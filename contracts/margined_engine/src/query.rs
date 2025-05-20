use cosmwasm_std::{Deps, Order, StdError, StdResult, Storage, Uint128};
use margined_common::integer::Integer;
use margined_perp::margined_engine::{
    ConfigResponse, LastPositionIdResponse, PauserResponse, PnlCalcOption, Position,
    PositionFilter, PositionTpSlResponse, PositionUnrealizedPnlResponse, Side, StateResponse,
    TradingConfigResponse,
};
use margined_utils::{
    contracts::helpers::{InsuranceFundController, VammController},
    tools::price_swap::get_output_price_with_reserves,
};

use crate::{
    contract::PAUSER,
    state::{
        read_config, read_last_position_id, read_position, read_positions,
        read_positions_with_indexer, read_state, read_trading_config, read_vamm_map,
        TmpReserveInfo, PREFIX_POSITION_BY_PRICE, PREFIX_POSITION_BY_SIDE,
        PREFIX_POSITION_BY_TRADER,
    },
    tick::query_ticks,
    utils::{
        calc_funding_payment, calc_remain_margin_with_funding_payment, calculate_tp_sl_spread,
        check_tp_sl_price, get_position_notional_unrealized_pnl, keccak_256, position_is_bad_debt,
        position_is_liquidated,
    },
};

type FilterFn = Box<dyn Fn(&Side) -> bool>;

/// Queries contract Config
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    read_config(deps.storage)
}

/// Queries contract Config
pub fn query_trading_config(deps: Deps) -> StdResult<TradingConfigResponse> {
    read_trading_config(deps.storage)
}

/// Queries contract State
pub fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = read_state(deps.storage)?;

    Ok(StateResponse {
        open_interest_notional: state.open_interest_notional,
        bad_debt: state.prepaid_bad_debt,
        pause: state.pause,
    })
}

/// Queries pauser from the admin
pub fn query_pauser(deps: Deps) -> StdResult<PauserResponse> {
    if let Some(pauser) = PAUSER.get(deps)? {
        Ok(PauserResponse { pauser })
    } else {
        Err(StdError::generic_err("No pauser set"))
    }
}

/// Queries user position
pub fn query_position(deps: Deps, vamm: String, position_id: u64) -> StdResult<Position> {
    // if vamm and trader are not correct, vamm_key will throw not found error
    let vamm_key = keccak_256(vamm.as_bytes());
    let position = read_position(deps.storage, &vamm_key, position_id)?;

    Ok(position)
}

/// Queries and returns users positions for registered vamms
pub fn query_positions(
    storage: &dyn Storage,
    vamm_key: &[u8],
    side: Option<Side>,
    filter: PositionFilter,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<i32>,
) -> StdResult<Vec<Position>> {
    let order_by = order_by.and_then(|val| Order::try_from(val).ok());

    let (direction_filter, direction_key): (FilterFn, Vec<u8>) = match side {
        // copy value to closure
        Some(d) => (Box::new(move |x| d.eq(x)), d.as_bytes().to_vec()),
        None => (Box::new(|_| true), Side::Buy.as_bytes().to_vec()),
    };

    let positions: Option<Vec<Position>> = match filter {
        PositionFilter::Trader(trader_addr) => read_positions_with_indexer::<Side>(
            storage,
            &[PREFIX_POSITION_BY_TRADER, vamm_key, trader_addr.as_bytes()],
            direction_filter,
            start_after,
            limit,
            order_by,
        )?,
        PositionFilter::Price(price) => {
            let price_key = price.to_be_bytes();
            read_positions_with_indexer::<Side>(
                storage,
                &[PREFIX_POSITION_BY_PRICE, vamm_key, &price_key],
                direction_filter,
                start_after,
                limit,
                order_by,
            )?
        }
        PositionFilter::None => match side {
            Some(_) => read_positions_with_indexer::<Side>(
                storage,
                &[PREFIX_POSITION_BY_SIDE, vamm_key, &direction_key],
                direction_filter,
                start_after,
                limit,
                order_by,
            )?,
            None => Some(read_positions(
                storage,
                vamm_key,
                start_after,
                limit,
                order_by,
            )?),
        },
    };

    Ok(positions.unwrap_or_default())
}

/// Queries user position
pub fn query_position_notional_unrealized_pnl(
    deps: Deps,
    vamm: String,
    position_id: u64,
    calc_option: PnlCalcOption,
) -> StdResult<PositionUnrealizedPnlResponse> {
    let vamm_key = keccak_256(vamm.as_bytes());
    // read the msg.senders position
    let position = read_position(deps.storage, &vamm_key, position_id)?;

    let result = get_position_notional_unrealized_pnl(deps, &position, calc_option)?;

    Ok(result)
}

/// Queries cumulative premium fractions
pub fn query_cumulative_premium_fraction(deps: Deps, vamm: String) -> StdResult<Integer> {
    // retrieve vamm data
    let vamm_map = read_vamm_map(deps.storage, &deps.api.addr_validate(&vamm)?)?;

    let result = match vamm_map.cumulative_premium_fractions.len() {
        0 => Integer::zero(),
        n => vamm_map.cumulative_premium_fractions[n - 1],
    };

    Ok(result)
}

/// Queries traders balance across all vamms with funding payment
pub fn query_trader_balance_with_funding_payment(
    deps: Deps,
    position_id: u64,
) -> StdResult<Uint128> {
    let config = read_config(deps.storage)?;

    let mut margin = Uint128::zero();

    let vamms = match config.insurance_fund {
        Some(insurance_fund) => {
            let insurance_controller = InsuranceFundController(insurance_fund);
            insurance_controller
                .all_vamms(&deps.querier, None)?
                .vamm_list
        }
        None => return Err(StdError::generic_err("insurance fund is not registered")),
    };

    for vamm in vamms.iter() {
        let position =
            query_trader_position_with_funding_payment(deps, vamm.to_string(), position_id)?;
        margin = margin.checked_add(position.margin)?;
    }

    Ok(margin)
}

/// Queries traders position across all vamms with funding payments
pub fn query_trader_position_with_funding_payment(
    deps: Deps,
    vamm: String,
    position_id: u64,
) -> StdResult<Position> {
    let config = read_config(deps.storage)?;

    let vamm_key = keccak_256(vamm.as_bytes());

    // retrieve latest user position
    let mut position = read_position(deps.storage, &vamm_key, position_id)?;

    let latest_cumulative_premium_fraction =
        query_cumulative_premium_fraction(deps, vamm.to_string())?;

    let funding_payment = calc_funding_payment(
        position.clone(),
        latest_cumulative_premium_fraction,
        config.decimals,
    );

    let margin_with_funding_payment = Integer::new_positive(position.margin) + funding_payment;

    if margin_with_funding_payment.is_positive() {
        position.margin = margin_with_funding_payment.value;
    } else {
        position.margin = Uint128::zero();
    }

    Ok(position)
}

/// Queries the margin ratio of a trader
pub fn query_margin_ratio(deps: Deps, position: &Position) -> StdResult<Integer> {
    if position.size.is_zero() {
        return Ok(Integer::zero());
    }

    let PositionUnrealizedPnlResponse {
        position_notional,
        unrealized_pnl,
    } = get_position_notional_unrealized_pnl(deps, position, PnlCalcOption::SpotPrice)?;

    let remain_margin = calc_remain_margin_with_funding_payment(deps, position, unrealized_pnl)?;

    let config = read_config(deps.storage)?;
    let margin_ratio = ((Integer::new_positive(remain_margin.margin)
        - Integer::new_positive(remain_margin.bad_debt))
        * Integer::new_positive(config.decimals))
        / Integer::new_positive(position_notional);

    Ok(margin_ratio)
}

/// Queries the withdrawable collateral of a trader
pub fn query_free_collateral(deps: Deps, vamm: String, position_id: u64) -> StdResult<Integer> {
    // retrieve the latest position
    let position = query_trader_position_with_funding_payment(deps, vamm.clone(), position_id)?;

    // get trader's unrealized PnL and choose the least beneficial one for the trader
    let PositionUnrealizedPnlResponse {
        position_notional: spot_notional,
        unrealized_pnl: spot_pnl,
    } = get_position_notional_unrealized_pnl(deps, &position, PnlCalcOption::SpotPrice)?;
    let PositionUnrealizedPnlResponse {
        position_notional: twap_notional,
        unrealized_pnl: twap_pnl,
    } = get_position_notional_unrealized_pnl(deps, &position, PnlCalcOption::Twap)?;

    // calculate and return margin
    let PositionUnrealizedPnlResponse {
        position_notional,
        unrealized_pnl,
    } = if spot_pnl.abs() > twap_pnl.abs() {
        PositionUnrealizedPnlResponse {
            position_notional: twap_notional,
            unrealized_pnl: twap_pnl,
        }
    } else {
        PositionUnrealizedPnlResponse {
            position_notional: spot_notional,
            unrealized_pnl: spot_pnl,
        }
    };

    // min(margin + funding, margin + funding + unrealized PnL) - position value * initMarginRatio
    let account_value = unrealized_pnl.checked_add(Integer::new_positive(position.margin))?;
    let minimum_collateral = if account_value
        .checked_sub(Integer::new_positive(position.margin))?
        .is_positive()
    {
        Integer::new_positive(position.margin)
    } else {
        account_value
    };

    // validate address inputs
    let vamm = deps.api.addr_validate(&vamm.clone())?;
    let vamm_controller = VammController(vamm.clone());
    let vamm_config = vamm_controller.config(&deps.querier)?;

    let margin_requirement = if position.size.is_positive() {
        position
            .notional
            .checked_mul(vamm_config.initial_margin_ratio)?
            .checked_div(vamm_config.decimals)?
    } else {
        position_notional
            .checked_mul(vamm_config.initial_margin_ratio)?
            .checked_div(vamm_config.decimals)?
    };

    Ok(minimum_collateral.checked_sub(Integer::new_positive(margin_requirement))?)
}

pub fn query_last_position_id(deps: Deps) -> StdResult<LastPositionIdResponse> {
    let last_position_id = read_last_position_id(deps.storage)?;
    let resp = LastPositionIdResponse { last_position_id };

    Ok(resp)
}

pub fn query_position_is_tpsl(
    deps: Deps,
    vamm: String,
    side: Side,
    do_tp: bool,
    limit: u32,
) -> StdResult<PositionTpSlResponse> {
    let config = read_config(deps.storage)?;
    let vamm_addr = deps.api.addr_validate(&vamm)?;
    let vamm_controller = VammController(vamm_addr.clone());
    let vamm_state = vamm_controller.state(&deps.querier).unwrap();
    let tmp_reserve = TmpReserveInfo {
        quote_asset_reserve: vamm_state.quote_asset_reserve,
        base_asset_reserve: vamm_state.base_asset_reserve,
    };

    let order_by = if do_tp == (side == Side::Buy) {
        Order::Descending
    } else {
        Order::Ascending
    };
    let vamm_key = keccak_256(vamm.as_bytes());

    let ticks = query_ticks(
        deps.storage,
        &vamm_key,
        side,
        None,
        Some(limit),
        Some(order_by.into()),
    )?;

    for tick in &ticks.ticks {
        let position_by_price = query_positions(
            deps.storage,
            &vamm_key,
            Some(side),
            PositionFilter::Price(tick.entry_price),
            None,
            None,
            Some(Order::Ascending.into()),
        )?;

        for position in &position_by_price {
            let base_asset_amount = position.size.value;
            let quote_asset_amount = get_output_price_with_reserves(
                &position.direction,
                base_asset_amount,
                tmp_reserve.quote_asset_reserve,
                tmp_reserve.base_asset_reserve,
            )?;
            let close_price = quote_asset_amount
                .checked_mul(config.decimals)?
                .checked_div(base_asset_amount)?;

            let stop_loss = position.stop_loss.unwrap_or_default();
            let take_profit = position.take_profit.unwrap_or_default();
            let (tp_spread, sl_spread) = calculate_tp_sl_spread(
                config.tp_sl_spread,
                take_profit,
                stop_loss,
                config.decimals,
            )?;
            let tp_sl_action = check_tp_sl_price(
                close_price,
                take_profit,
                stop_loss,
                tp_spread,
                sl_spread,
                &position.side,
            )?;

            let tp_sl_flag = if do_tp {
                tp_sl_action == "trigger_take_profit"
            } else {
                tp_sl_action == "trigger_stop_loss"
            };

            if tp_sl_flag {
                return Ok(PositionTpSlResponse { is_tpsl: true });
            }
        }
    }

    Ok(PositionTpSlResponse { is_tpsl: false })
}

pub fn query_position_is_bad_debt(deps: Deps, position_id: u64, vamm: String) -> StdResult<bool> {
    let vamm_key = keccak_256(vamm.as_bytes());
    let vamm_addr = deps.api.addr_validate(&vamm)?;
    let vamm_controller = VammController(vamm_addr.clone());
    let vamm_state = vamm_controller.state(&deps.querier)?;
    let position = read_position(deps.storage, &vamm_key, position_id)?;
    let is_bad_debt = position_is_bad_debt(
        deps,
        &position,
        vamm_state.quote_asset_reserve,
        vamm_state.base_asset_reserve,
    )?;
    Ok(is_bad_debt)
}

pub fn query_position_is_liquidated(deps: Deps, position_id: u64, vamm: String) -> StdResult<bool> {
    let config = read_config(deps.storage)?;
    let vamm_key = keccak_256(vamm.as_bytes());
    let vamm_addr = deps.api.addr_validate(&vamm)?;
    let vamm_controller = VammController(vamm_addr.clone());
    let position = read_position(deps.storage, &vamm_key, position_id)?;
    let is_liquidated = position_is_liquidated(
        deps,
        &position,
        config.maintenance_margin_ratio,
        &vamm_controller,
    )?;
    Ok(is_liquidated)
}
