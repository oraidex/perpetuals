use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError, Uint128};

use crate::{
    contract::{EXECUTOR, OWNER},
    error::ContractError,
    state::store_price_data,
};

pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<Response, ContractError> {
    // validate the address
    let valid_owner = deps.api.addr_validate(&owner)?;

    Ok(OWNER.execute_update_admin(deps, info, Some(valid_owner))?)
}

/// this is a mock function that enables storage of data
/// by the contract owner will be replaced by integration
/// with on-chain price oracles in the future.
pub fn append_price(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    key: String,
    price: Uint128,
    timestamp: u64,
) -> Result<Response, ContractError> {
    // check permission
    EXECUTOR.assert_admin(deps.as_ref(), &info.sender)?;

    if price.is_zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "Price is must not be zero",
        )));
    }

    if timestamp > env.block.time.seconds() || timestamp == 0u64 {
        return Err(ContractError::Std(StdError::generic_err(
            "Invalid timestamp",
        )));
    }
    store_price_data(deps.storage, key, price, timestamp)?;

    Ok(Response::default().add_attribute("action", "append_price"))
}

/// this is a mock function that enables storage of data
/// by the contract owner will be replaced by integration
/// with on-chain price oracles in the future.
pub fn append_multiple_price(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    key: String,
    prices: Vec<Uint128>,
    timestamps: Vec<u64>,
) -> Result<Response, ContractError> {
    // check permission
    EXECUTOR.assert_admin(deps.as_ref(), &info.sender)?;

    // This throws if the prices and timestamps are not the same length
    if prices.len() != timestamps.len() {
        return Err(ContractError::Std(StdError::generic_err(
            "Prices and timestamps are not the same length",
        )));
    }

    for index in 0..prices.len() {
        if prices[index].is_zero() {
            return Err(ContractError::Std(StdError::generic_err(
                "Price is must not be zero",
            )));
        }

        if timestamps[index] > env.block.time.seconds() || timestamps[index] == 0u64 {
            return Err(ContractError::Std(StdError::generic_err(
                "Invalid timestamp",
            )));
        }
        store_price_data(deps.storage, key.clone(), prices[index], timestamps[index])?;
    }

    Ok(Response::default().add_attribute("action", "append_multiple_price"))
}

pub fn update_executor(
    deps: DepsMut,
    info: MessageInfo,
    executor: String,
) -> Result<Response, ContractError> {
    OWNER.assert_admin(deps.as_ref(), &info.sender)?;

    // validate the address
    let valid_executor = deps.api.addr_validate(&executor)?;

    EXECUTOR.set(deps, Some(valid_executor))?;

    Ok(Response::new().add_attributes(vec![
        ("action", "update_executor"),
        ("new_executor", &executor),
    ]))
}
