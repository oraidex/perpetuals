use crate::error::ContractError;
use crate::handle::update_executor;
use crate::query::{query_executor, query_get_price_detail, query_last_round_id};
use crate::{
    handle::{append_multiple_price, append_price, update_owner},
    query::{
        query_config, query_get_previous_price, query_get_price, query_get_twap_price, query_owner,
    },
    state::{store_config, Config},
};
use cw2::set_contract_version;
use cw_controllers::Admin;

use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use margined_perp::margined_pricefeed::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "crates.io:margined-pricefeed";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Owner admin
pub const OWNER: Admin = Admin::new("owner");

pub const EXECUTOR: Admin = Admin::new("executor");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {};

    store_config(deps.storage, &config)?;

    OWNER.set(deps, Some(info.sender))?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AppendPrice {
            key,
            price,
            timestamp,
        } => append_price(deps, env, info, key, price, timestamp),
        ExecuteMsg::AppendMultiplePrice {
            key,
            prices,
            timestamps,
        } => append_multiple_price(deps, env, info, key, prices, timestamps),
        ExecuteMsg::UpdateOwner { owner } => update_owner(deps, info, owner),
        ExecuteMsg::UpdateExecutor { executor } => update_executor(deps, info, executor),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::GetOwner {} => to_json_binary(&query_owner(deps)?),
        QueryMsg::GetPrice { key } => to_json_binary(&query_get_price(deps, key)?),
        QueryMsg::GetPreviousPrice {
            key,
            num_round_back,
        } => to_json_binary(&query_get_previous_price(deps, key, num_round_back)?),
        QueryMsg::GetTwapPrice { key, interval } => {
            to_json_binary(&query_get_twap_price(deps, env, key, interval)?)
        }
        QueryMsg::GetLastRoundId { key } => to_json_binary(&query_last_round_id(deps, key)?),
        QueryMsg::GetExecutor {} => to_json_binary(&query_executor(deps)?),
        QueryMsg::GetPriceDetail { key } => to_json_binary(&query_get_price_detail(deps, key)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new())
}
