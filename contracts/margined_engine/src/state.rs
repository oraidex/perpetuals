use cosmwasm_schema::cw_serde;
use cosmwasm_schema::serde::{de::DeserializeOwned, Serialize};
use cosmwasm_std::{
    from_slice, to_vec, Addr, Order as OrderBy, StdError, StdResult, Storage, Uint128,
};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};
use std::cmp::Ordering;

use margined_common::{asset::Asset, integer::Integer};
use margined_perp::margined_engine::{
    ConfigResponse, PauseType, Position, Side, TradingConfigResponse,
};

use crate::utils::calc_range_start;

// settings for pagination
pub const MAX_LIMIT: u32 = 100;
pub const DEFAULT_LIMIT: u32 = 10;

pub static KEY_TRADING_CONFIG: &[u8] = b"trading_config";
pub static KEY_CONFIG: &[u8] = b"config";
pub static KEY_STATE: &[u8] = b"state";
pub static KEY_SENT_FUNDS: &[u8] = b"sent-funds";
pub static KEY_TMP_SWAP: &[u8] = b"tmp-swap";
pub static KEY_TMP_LIQUIDATOR: &[u8] = b"tmp-liquidator";
pub static KEY_VAMM_MAP: &[u8] = b"vamm-map";
pub static KEY_LAST_POSITION_ID: &[u8] = b"last_position_id";

static PREFIX_POSITION: &[u8] = b"position"; // prefix position
pub static PREFIX_POSITION_BY_SIDE: &[u8] = b"position_by_direction"; // position from the direction
pub static PREFIX_POSITION_BY_PRICE: &[u8] = b"position_by_price"; // position from the price
pub static PREFIX_POSITION_BY_TRADER: &[u8] = b"position_by_trader"; // position from a trader
pub static PREFIX_TICK: &[u8] = b"tick"; // this is tick with value is the total positions

pub type Config = ConfigResponse;
pub type TradingConfig = TradingConfigResponse;

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    storage.set(KEY_CONFIG, &to_vec(config)?);
    Ok(())
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    match storage.get(KEY_CONFIG) {
        Some(data) => from_slice(&data),
        None => Err(StdError::generic_err("Config not found")),
    }
}

pub fn store_trading_config(storage: &mut dyn Storage, config: &TradingConfig) -> StdResult<()> {
    storage.set(KEY_TRADING_CONFIG, &to_vec(config)?);
    Ok(())
}

pub fn read_trading_config(storage: &dyn Storage) -> StdResult<TradingConfig> {
    match storage.get(KEY_TRADING_CONFIG) {
        Some(data) => from_slice(&data),
        None => Err(StdError::generic_err("Trading config not found")),
    }
}
#[cw_serde]
pub struct State {
    pub open_interest_notional: Uint128,
    pub prepaid_bad_debt: Uint128,
    pub pause: PauseType,
}

pub fn init_last_position_id(storage: &mut dyn Storage) -> StdResult<()> {
    singleton(storage, KEY_LAST_POSITION_ID).save(&0u64)
}

pub fn increase_last_position_id(storage: &mut dyn Storage) -> StdResult<u64> {
    singleton(storage, KEY_LAST_POSITION_ID).update(|v| Ok(v + 1))
}

pub fn read_last_position_id(storage: &dyn Storage) -> StdResult<u64> {
    singleton_read(storage, KEY_LAST_POSITION_ID).load()
}

pub fn store_state(storage: &mut dyn Storage, state: &State) -> StdResult<()> {
    storage.set(KEY_STATE, &to_vec(state)?);
    Ok(())
}

pub fn read_state(storage: &dyn Storage) -> StdResult<State> {
    match storage.get(KEY_STATE) {
        Some(data) => from_slice(&data),
        None => Err(StdError::generic_err("State not found")),
    }
}

pub fn store_position(
    storage: &mut dyn Storage,
    key: &[u8],
    position: &Position,
    inserted: bool,
) -> StdResult<u64> {
    let position_id_key = &position.position_id.to_be_bytes();
    let price_key = position.entry_price.to_be_bytes();
    Bucket::multilevel(storage, &[PREFIX_POSITION, key]).save(position_id_key, position)?;

    let tick_namespaces = &[PREFIX_TICK, key, position.side.as_bytes()];
    // first time then total is 0
    let mut total_tick_orders = ReadonlyBucket::<u64>::multilevel(storage, tick_namespaces)
        .load(&price_key)
        .unwrap_or_default();

    if inserted {
        total_tick_orders += 1;
    }

    // save total orders for a tick
    Bucket::multilevel(storage, tick_namespaces).save(&price_key, &total_tick_orders)?;

    Bucket::multilevel(
        storage,
        &[PREFIX_POSITION_BY_TRADER, key, position.trader.as_bytes()],
    )
    .save(position_id_key, &position.side)?;

    Bucket::multilevel(
        storage,
        &[PREFIX_POSITION_BY_SIDE, key, &position.side.as_bytes()],
    )
    .save(position_id_key, &position.side)?;

    Bucket::multilevel(storage, &[PREFIX_POSITION_BY_PRICE, key, &price_key])
        .save(position_id_key, &position.side)?;

    Ok(total_tick_orders)
}

pub fn remove_position(
    storage: &mut dyn Storage,
    key: &[u8],
    position: &Position,
) -> StdResult<u64> {
    let position_id_key = &position.position_id.to_be_bytes();
    let price_key = position.entry_price.to_be_bytes();

    Bucket::<Position>::multilevel(storage, &[PREFIX_POSITION, key]).remove(position_id_key);

    // not found means total is 0
    let tick_namespaces = &[PREFIX_TICK, key, position.side.as_bytes()];
    let mut total_tick_orders = ReadonlyBucket::<u64>::multilevel(storage, tick_namespaces)
        .load(&price_key)
        .unwrap_or_default();

    // substract one order, if total is 0 mean not existed
    if total_tick_orders > 0 {
        total_tick_orders -= 1;
        if total_tick_orders > 0 {
            // save total orders for a tick
            Bucket::multilevel(storage, tick_namespaces).save(&price_key, &total_tick_orders)?;
        } else {
            Bucket::<u64>::multilevel(storage, tick_namespaces).remove(&price_key);
        }
    }

    Bucket::<Side>::multilevel(
        storage,
        &[PREFIX_POSITION_BY_TRADER, key, position.trader.as_bytes()],
    )
    .remove(position_id_key);

    Bucket::<Side>::multilevel(
        storage,
        &[PREFIX_POSITION_BY_SIDE, key, &position.side.as_bytes()],
    )
    .remove(position_id_key);

    Bucket::<Side>::multilevel(storage, &[PREFIX_POSITION_BY_PRICE, key, &price_key])
        .remove(position_id_key);

    // return total orders belong to the tick
    Ok(total_tick_orders)
}

pub fn read_position(storage: &dyn Storage, key: &[u8], position_id: u64) -> StdResult<Position> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_POSITION, key]).load(&position_id.to_be_bytes())
}

/// read_positions_with_indexer: namespace is PREFIX + KEY + INDEXER
pub fn read_positions_with_indexer<T: Serialize + DeserializeOwned>(
    storage: &dyn Storage,
    namespaces: &[&[u8]],
    filter: Box<dyn Fn(&T) -> bool>,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<Option<Vec<Position>>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_after = start_after.map(|id| id.to_be_bytes().to_vec());
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Ascending) => (calc_range_start(start_after), None, OrderBy::Ascending),
        _ => (None, start_after, OrderBy::Descending),
    };

    // just get 1 byte of value is ok
    let position_indexer: ReadonlyBucket<T> = ReadonlyBucket::multilevel(storage, namespaces);
    let order_bucket = ReadonlyBucket::multilevel(storage, &[PREFIX_POSITION, namespaces[1]]);

    position_indexer
        .range(start.as_deref(), end.as_deref(), order_by)
        .filter(|item| item.as_ref().map_or(false, |item| filter(&item.1)))
        .take(limit)
        .map(|item| order_bucket.load(&item?.0))
        .collect()
}

pub fn read_positions(
    storage: &dyn Storage,
    key: &[u8],
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<Vec<Position>> {
    let position_bucket: ReadonlyBucket<Position> =
        ReadonlyBucket::multilevel(storage, &[PREFIX_POSITION, key]);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_after = start_after.map(|id| id.to_be_bytes().to_vec());
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Ascending) => (calc_range_start(start_after), None, OrderBy::Ascending),
        _ => (None, start_after, OrderBy::Descending),
    };

    position_bucket
        .range(start.as_deref(), end.as_deref(), order_by)
        .take(limit)
        .map(|item| item.map(|item| item.1))
        .collect()
}

/// Used to monitor that transferred native tokens are sufficient when opening a
/// new position or relevant operations
#[cw_serde]
pub struct SentFunds {
    pub asset: Asset,
    pub required: Uint128,
}

impl SentFunds {
    /// throws an error if the required funds is less than the asset amount
    pub fn are_sufficient(&self) -> StdResult<()> {
        // this should only pass if asset.amount == required
        match self.asset.amount.cmp(&self.required) {
            Ordering::Greater => Err(StdError::generic_err("sent funds are excessive")),
            Ordering::Less => Err(StdError::generic_err("sent funds are insufficient")),
            _ => Ok(()),
        }
    }
}

pub fn store_sent_funds(storage: &mut dyn Storage, funds: &SentFunds) -> StdResult<()> {
    storage.set(KEY_SENT_FUNDS, &to_vec(funds)?);
    Ok(())
}

pub fn remove_sent_funds(storage: &mut dyn Storage) {
    storage.remove(KEY_SENT_FUNDS)
}

pub fn read_sent_funds(storage: &dyn Storage) -> StdResult<SentFunds> {
    match storage.get(KEY_SENT_FUNDS) {
        Some(data) => from_slice(&data),
        None => Err(StdError::generic_err("No sent funds are stored")),
    }
}

#[cw_serde]
pub struct TmpSwapInfo {
    pub position_id: u64,
    pub vamm: Addr,
    pub pair: String,
    pub trader: Addr,
    pub side: Side,                   // buy or sell
    pub margin_amount: Uint128,       // amount of quote asset being supplied
    pub leverage: Uint128,            // leverage of new position
    pub open_notional: Uint128,       // notional of position being opened
    pub position_notional: Uint128,   // notional of existing position, inclusing funding
    pub unrealized_pnl: Integer,      // any pnl due
    pub margin_to_vault: Integer,     // margin to be sent to vault
    pub take_profit: Option<Uint128>, // take profit price of position
    pub stop_loss: Option<Uint128>,   // stop loss price of position
    pub spread_fee: Uint128,          // spread fee
    pub toll_fee: Uint128,            // toll fee
}

pub fn store_tmp_swap(storage: &mut dyn Storage, swap: &TmpSwapInfo) -> StdResult<()> {
    let position_id_key = &swap.position_id.to_be_bytes();
    Bucket::new(storage, KEY_TMP_SWAP).save(position_id_key, swap)?;
    Ok(())
}

pub fn remove_tmp_swap(storage: &mut dyn Storage, position_id_key: &[u8]) {
    Bucket::<'_, TmpSwapInfo>::new(storage, KEY_TMP_SWAP).remove(position_id_key)
}

pub fn read_tmp_swap(storage: &dyn Storage, position_id_key: &[u8]) -> StdResult<TmpSwapInfo> {
    ReadonlyBucket::new(storage, KEY_TMP_SWAP).load(position_id_key)
}

pub fn store_tmp_liquidator(storage: &mut dyn Storage, liquidator: &Addr) -> StdResult<()> {
    storage.set(KEY_TMP_LIQUIDATOR, &to_vec(liquidator)?);
    Ok(())
}

pub fn remove_tmp_liquidator(storage: &mut dyn Storage) {
    storage.remove(KEY_TMP_LIQUIDATOR)
}

pub fn read_tmp_liquidator(storage: &dyn Storage) -> StdResult<Addr> {
    match storage.get(KEY_TMP_LIQUIDATOR) {
        Some(data) => from_slice(&data),
        None => Err(StdError::generic_err("Addr not found")),
    }
}

#[cw_serde]
pub struct TmpReserveInfo {
    pub quote_asset_reserve: Uint128,
    pub base_asset_reserve: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct VammMap {
    pub last_restriction_block: u64,
    pub cumulative_premium_fractions: Vec<Integer>,
}

pub fn store_vamm_map(storage: &mut dyn Storage, vamm: Addr, vamm_map: &VammMap) -> StdResult<()> {
    storage.set(
        &[KEY_VAMM_MAP, vamm.as_bytes()].concat(),
        &to_vec(vamm_map)?,
    );
    Ok(())
}

pub fn read_vamm_map(storage: &dyn Storage, vamm: &Addr) -> StdResult<VammMap> {
    match storage.get(&[KEY_VAMM_MAP, vamm.as_bytes()].concat()) {
        Some(data) => from_slice(&data),
        None => Ok(VammMap::default()),
    }
}

/// Accumulates the premium fractions at each settlement payment so that eventually users take
/// their P&L
pub fn append_cumulative_premium_fraction(
    storage: &mut dyn Storage,
    vamm: Addr,
    premium_fraction: Integer,
) -> StdResult<Integer> {
    let mut vamm_map = read_vamm_map(storage, &vamm)?;
    let mut latest_premium_fraction = premium_fraction;
    // we push the first premium fraction to an empty array
    // else we add them together prior to pushing
    match vamm_map.cumulative_premium_fractions.len() {
        0 => vamm_map.cumulative_premium_fractions.push(premium_fraction),
        n => {
            let current_premium_fraction = vamm_map.cumulative_premium_fractions[n - 1];
            latest_premium_fraction = premium_fraction + current_premium_fraction;

            vamm_map
                .cumulative_premium_fractions
                .push(latest_premium_fraction)
        }
    }

    store_vamm_map(storage, vamm, &vamm_map)?;

    Ok(latest_premium_fraction)
}

pub fn enter_restriction_mode(
    storage: &mut dyn Storage,
    vamm: Addr,
    block_height: u64,
) -> StdResult<()> {
    let mut vamm_map = read_vamm_map(storage, &vamm)?;

    vamm_map.last_restriction_block = block_height;

    store_vamm_map(storage, vamm, &vamm_map)
}


