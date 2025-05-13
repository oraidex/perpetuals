use cosmwasm_std::{Addr, Deps, StdError, StdResult};
use margined_perp::margined_insurance_fund::{
    AllVammResponse, AllVammStatusResponse, ConfigResponse, OwnerResponse, VammResponse,
    VammStatusResponse,
};
use margined_utils::contracts::helpers::VammController;

use crate::{
    contract::OWNER,
    state::{is_vamm, read_config, read_vammlist},
};

const DEFAULT_PAGINATION_LIMIT: u32 = 10u32;
pub const MAX_PAGINATION_LIMIT: u32 = 100u32;

/// Queries contract owner from the admin
pub fn query_owner(deps: Deps) -> StdResult<OwnerResponse> {
    if let Some(owner) = OWNER.get(deps)? {
        Ok(OwnerResponse { owner })
    } else {
        Err(StdError::generic_err("No owner set"))
    }
}

/// Queries contract config
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    read_config(deps.storage)
}

/// Queries if the vAMM with given address is already stored
pub fn query_is_vamm(deps: Deps, vamm: String) -> StdResult<VammResponse> {
    // validate address
    let vamm_valid = deps.api.addr_validate(&vamm)?;

    // read the current storage and pull the vamm status corresponding to the given addr
    let vamm_bool = is_vamm(deps.storage, vamm_valid);

    Ok(VammResponse { is_vamm: vamm_bool })
}

/// Queries the list of vAMMs currently stored (not necessarily on)
pub fn query_all_vamm(deps: Deps, limit: Option<u32>) -> StdResult<AllVammResponse> {
    // set the limit for pagination
    let limit = limit
        .unwrap_or(DEFAULT_PAGINATION_LIMIT)
        .min(MAX_PAGINATION_LIMIT) as usize;

    let list = read_vammlist(deps.storage, limit)?;
    Ok(AllVammResponse { vamm_list: list })
}

/// Queries the status of the vAMM with given address
pub fn query_vamm_status(deps: Deps, vamm: String) -> StdResult<VammStatusResponse> {
    // validate address
    let vamm_valid = deps.api.addr_validate(&vamm)?;

    let vamm_controller = VammController(vamm_valid);

    // query the vamms current status
    let vamm_bool = vamm_controller.state(&deps.querier)?.open;

    Ok(VammStatusResponse {
        vamm_status: vamm_bool,
    })
}

/// Queries the status of multiple vAMMs, returning the vAMM address and whether it is on/off
pub fn query_status_all_vamm(deps: Deps, limit: Option<u32>) -> StdResult<AllVammStatusResponse> {
    // set the limit for pagination
    let limit = limit.unwrap_or(DEFAULT_PAGINATION_LIMIT) as usize;
    // .min(MAX_PAGINATION_LIMIT) as usize;

    let mut status_list: Vec<(Addr, bool)> = vec![];

    // iterate through the vamm list and query the status one by one
    for vamm in read_vammlist(deps.storage, limit)? {
        let vamm_controller = VammController(vamm.clone());
        let vamm_bool = vamm_controller.state(&deps.querier)?.open;
        status_list.push((vamm, vamm_bool));
    }

    Ok(AllVammStatusResponse {
        vamm_list_status: status_list,
    })
}
