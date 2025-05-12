use cosmwasm_std::{Addr, Deps, DepsMut, MessageInfo, Response, StdError, StdResult};
use cw_storage_plus::Map;

use crate::state::{read_config, read_trading_config};

/// Whitelisted trader can open position
pub const WHITELIST_TRADER: Map<Addr, bool> = Map::new("whitelist_trader");
/// relayer
pub const RELAYER: Map<Addr, bool> = Map::new("relayer");

// function to set relayer
// only owner can set relayer
pub fn set_relayer(deps: DepsMut, info: MessageInfo, relayers: Vec<Addr>) -> StdResult<Response> {
    let config = read_config(deps.storage)?;
    if config.owner != info.sender {
        return Err(StdError::generic_err("Unauthorized"));
    }

    for relayer in relayers {
        RELAYER.save(deps.storage, relayer, &true)?;
    }

    Ok(Response::new().add_attribute("action", "set_relayer"))
}

// function to remove relayer
// only owner can remove relayer
pub fn remove_relayer(
    deps: DepsMut,
    info: MessageInfo,
    relayers: Vec<Addr>,
) -> StdResult<Response> {
    let config = read_config(deps.storage)?;
    if config.owner != info.sender {
        return Err(StdError::generic_err("Unauthorized"));
    }

    for relayer in relayers {
        RELAYER.remove(deps.storage, relayer);
    }

    Ok(Response::new().add_attribute("action", "remove_relayer"))
}

// function to whitelist trader
// only relayer can whitelist trader
pub fn whitelist_trader(
    deps: DepsMut,
    info: MessageInfo,
    traders: Vec<Addr>,
) -> StdResult<Response> {
    if !RELAYER
        .may_load(deps.storage, info.sender)?
        .unwrap_or(false)
    {
        return Err(StdError::generic_err("Unauthorized"));
    }

    for trader in traders {
        WHITELIST_TRADER.save(deps.storage, trader, &true)?;
    }

    Ok(Response::new().add_attribute("action", "whitelist_trader"))
}

// function to remove whitelist trader
// only relayer can remove whitelist trader
pub fn remove_whitelist_trader(
    deps: DepsMut,
    info: MessageInfo,
    traders: Vec<Addr>,
) -> StdResult<Response> {
    if !RELAYER
        .may_load(deps.storage, info.sender)?
        .unwrap_or(false)
    {
        return Err(StdError::generic_err("Unauthorized"));
    }

    for trader in traders {
        WHITELIST_TRADER.remove(deps.storage, trader);
    }

    Ok(Response::new().add_attribute("action", "remove_whitelist_trader"))
}

pub fn is_whitelisted(deps: Deps, trader: Addr) -> StdResult<Response> {
    let trading_config = read_trading_config(deps.storage)?;
    if !trading_config.enable_whitelist {
        return Ok(Response::new());
    }

    if WHITELIST_TRADER
        .may_load(deps.storage, trader)?
        .unwrap_or(false)
    {
        return Ok(Response::new());
    }

    Err(StdError::generic_err("Unauthorized"))
}
