use crate::{
    contract::OWNER,
    state::{
        read_config, read_swap_info, read_vammlist, remove_vamm as remove_amm, save_vamm,
        VAMM_LIMIT,
    },
};
use cosmwasm_std::{
    to_binary, CosmosMsg, Decimal, Deps, DepsMut, Env, Fraction, MessageInfo, Response, StdError,
    StdResult, Uint128, WasmMsg,
};

use cw20::Cw20ExecuteMsg;
use margined_common::{asset::AssetInfo, messages::wasm_execute};
use margined_perp::margined_vamm::ExecuteMsg as VammExecuteMessage;
use margined_utils::contracts::helpers::{EngineController, SmartRouterController, VammController};

pub fn update_owner(deps: DepsMut, info: MessageInfo, owner: String) -> StdResult<Response> {
    // validate the address
    let valid_owner = deps.api.addr_validate(&owner)?;

    OWNER
        .execute_update_admin(deps, info, Some(valid_owner))
        .map_err(|error| StdError::generic_err(error.to_string()))
}

pub fn add_vamm(deps: DepsMut, info: MessageInfo, vamm: String) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    // check permission
    if !OWNER.is_admin(deps.as_ref(), &info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    // validate address
    let vamm_valid = deps.api.addr_validate(&vamm)?;

    let vamm_controller = VammController(vamm_valid.clone());
    let engine_controller = EngineController(config.engine);

    // check decimals are consistent
    let engine_decimals = engine_controller.config(&deps.querier)?.decimals;
    let vamm_decimals = vamm_controller.config(&deps.querier)?.decimals;

    if engine_decimals != vamm_decimals {
        return Err(StdError::generic_err(
            "vAMM decimals incompatible with margin engine",
        ));
    }

    // add the amm
    save_vamm(deps.storage, vamm_valid)?;

    Ok(Response::default().add_attribute("action", "add_vamm"))
}

pub fn remove_vamm(deps: DepsMut, info: MessageInfo, vamm: String) -> StdResult<Response> {
    // check permission
    if !OWNER.is_admin(deps.as_ref(), &info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    // validate address
    let vamm_valid = deps.api.addr_validate(&vamm)?;

    // remove vamm here
    remove_amm(deps.storage, vamm_valid)?;

    Ok(Response::default().add_attribute("action", "remove_amm"))
}

pub fn shutdown_all_vamm(deps: DepsMut, _env: Env, info: MessageInfo) -> StdResult<Response> {
    // check permission
    if !OWNER.is_admin(deps.as_ref(), &info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    // construct all the shutdown messages
    let keys = read_vammlist(deps.storage, VAMM_LIMIT)?;

    // initialise the submsgs vec
    let mut msgs = vec![];
    for vamm in keys.iter() {
        let msg = wasm_execute(vamm, &VammExecuteMessage::SetOpen { open: false }, vec![])?;
        msgs.push(msg);
    }

    Ok(Response::default()
        .add_messages(msgs)
        .add_attribute("action", "shutdown_all_vamm"))
}

pub fn withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token: AssetInfo,
    amount: Uint128,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;

    // check permission
    if info.sender != config.engine {
        return Err(StdError::generic_err("unauthorized"));
    }

    let mut msgs: Vec<CosmosMsg> = vec![];

    // if eligible_collateral token balance can't afford the debt amount, ask staking token to mint
    let remain_amount = token.query_balance(&deps.querier, env.contract.address.clone())?;
    // TODO: swap staking token to amount token using amm rounter
    if remain_amount < amount {
        // deposit_token is perp token, reward token default is perp token, and receive fee token distribution as well
        // insurance_fund contract is the minter of perp token
        msgs.extend(mint_and_swap_for_loss(
            deps.as_ref(),
            env,
            token.clone(),
            amount - remain_amount,
        )?);
    }

    // send tokens if native or cw20

    msgs.push(token.into_msg(config.engine.to_string(), amount, None)?);

    Ok(Response::default().add_messages(msgs).add_attributes(vec![
        ("action", "insurance_withdraw"),
        ("amount", &amount.to_string()),
    ]))
}

pub fn mint_and_swap_for_loss(
    deps: Deps,
    env: Env,
    ask_token: AssetInfo,
    ask_amount: Uint128,
) -> StdResult<Vec<CosmosMsg>> {
    let config = read_config(deps.storage)?;
    let swap_info = read_swap_info(deps.storage)?;
    let smart_router = SmartRouterController(swap_info.smart_router.to_string());
    let perp_token = AssetInfo::Token {
        contract_addr: config.perp_token.clone(),
    };

    let simulate_swap = smart_router.build_swap_operations(
        &deps.querier,
        ask_token.clone(),
        perp_token.clone(),
        Some(ask_amount),
    )?;
    let belief_price = smart_router.simulate_belief_price(
        &deps.querier,
        ask_token.clone(),
        perp_token.clone(),
        swap_info.swap_fee,
    )?;
    let expected_return = ask_amount * belief_price;
    let price_impact = Decimal::from_ratio(simulate_swap.actual_minimum_receive, expected_return);

    let perp_token_require = expected_return
        * (price_impact.inv().unwrap()
            + config.additional_mint_rate
            + Decimal::from_ratio(simulate_swap.swap_ops.len() as u128, 1u128));

    let simulate_swap = smart_router.build_swap_operations(
        &deps.querier,
        perp_token.clone(),
        ask_token.clone(),
        Some(perp_token_require),
    )?;

    Ok(vec![
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.perp_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: env.contract.address.to_string(),
                amount: perp_token_require,
            })?,
            funds: vec![],
        }),
        smart_router.execute_operations(
            swap_info.swap_router.to_string(),
            perp_token,
            perp_token_require,
            simulate_swap.swap_ops,
            Some(ask_amount),
            None,
        )?,
    ])
}
