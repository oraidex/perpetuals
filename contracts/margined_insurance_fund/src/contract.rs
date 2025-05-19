#[cfg(not(feature = "library"))]
use crate::error::ContractError;
use crate::{
    handle::{
        add_vamm, remove_vamm, shutdown_all_vamm, update_owner, update_relayer, withdraw,
        withdraw_fund,
    },
    query::{
        query_all_vamm, query_config, query_is_vamm, query_owner, query_status_all_vamm,
        query_vamm_status,
    },
    state::{store_config, Config},
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use cw_controllers::Admin;
use margined_perp::margined_insurance_fund::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "crates.io:margined-insurance-fund";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Owner admin
pub const OWNER: Admin = Admin::new("owner");
/// relayer
pub const RELAYER: Admin = Admin::new("relayer");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        engine: deps.api.addr_validate(&msg.engine)?,
    };

    store_config(deps.storage, &config)?;

    OWNER.set(deps, Some(info.sender))?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateOwner { owner } => update_owner(deps, info, owner),
        ExecuteMsg::UpdateRelayer { relayer } => update_relayer(deps, info, relayer),
        ExecuteMsg::AddVamm { vamm } => add_vamm(deps, info, vamm),
        ExecuteMsg::RemoveVamm { vamm } => remove_vamm(deps, info, vamm),
        ExecuteMsg::Withdraw { token, amount } => withdraw(deps, info, token, amount),
        ExecuteMsg::ShutdownVamms {} => shutdown_all_vamm(deps, env, info),
        ExecuteMsg::WithdrawFund { token, amount } => withdraw_fund(deps, info, token, amount),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::GetOwner {} => to_binary(&query_owner(deps)?),
        QueryMsg::IsVamm { vamm } => to_binary(&query_is_vamm(deps, vamm)?),
        QueryMsg::GetAllVamm { limit } => to_binary(&query_all_vamm(deps, limit)?),
        QueryMsg::GetVammStatus { vamm } => to_binary(&query_vamm_status(deps, vamm)?),
        QueryMsg::GetAllVammStatus { limit } => to_binary(&query_status_all_vamm(deps, limit)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new())
}
