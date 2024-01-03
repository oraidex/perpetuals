use crate::error::ContractError;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    attr, ensure, ensure_eq, Addr, Deps, DepsMut, Env, Event, MessageInfo, Response, StdResult,
};
use cw_controllers::Admin;
use cw_storage_plus::Item;

pub const MAX_DURATION: u64 = 604800u64;

#[cw_serde]
pub struct OwnerProposal {
    pub owner: Addr,
    pub expiry: u64,
}

pub fn handle_ownership_proposal(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    proposed_owner: String,
    duration: u64,
    owner: Admin,
    proposal: Item<OwnerProposal>,
) -> Result<Response, ContractError> {
    ensure!(
        owner.is_admin(deps.as_ref(), &info.sender)?,
        ContractError::Unauthorized {}
    );

    let proposed_owner = deps.api.addr_validate(proposed_owner.as_str())?;

    ensure!(
        !owner.is_admin(deps.as_ref(), &proposed_owner)?,
        ContractError::InvalidOwnership {}
    );

    if MAX_DURATION < duration {
        return Err(ContractError::InvalidDuration(MAX_DURATION));
    }

    let expiry = env.block.time.seconds() + duration;

    proposal.save(
        deps.storage,
        &OwnerProposal {
            owner: proposed_owner.clone(),
            expiry,
        },
    )?;

    let proposal_event = Event::new("propose_proposed_owner").add_attributes(vec![
        attr("proposed_owner", proposed_owner),
        attr("expiry", expiry.to_string()),
    ]);

    Ok(Response::new().add_event(proposal_event))
}

pub fn handle_ownership_proposal_rejection(
    deps: DepsMut,
    info: MessageInfo,
    owner: Admin,
    proposal: Item<OwnerProposal>,
) -> Result<Response, ContractError> {
    ensure!(
        owner.is_admin(deps.as_ref(), &info.sender)?,
        ContractError::Unauthorized {}
    );

    proposal.remove(deps.storage);

    let reject_proposal_event = Event::new("reject_ownership");

    Ok(Response::new().add_event(reject_proposal_event))
}

pub fn handle_claim_ownership(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    owner: Admin,
    proposal: Item<OwnerProposal>,
) -> Result<Response, ContractError> {
    let p = proposal
        .load(deps.storage)
        .map_err(|_| ContractError::ProposalNotFound {})?;

    ensure_eq!(p.owner, &info.sender, ContractError::Unauthorized {});

    if env.block.time.seconds() > p.expiry {
        return Err(ContractError::Expired {});
    }

    let new_owner = p.owner;

    proposal.remove(deps.storage);

    owner.set(deps, Some(new_owner.clone()))?;

    let reject_proposal_event =
        Event::new("update_owner").add_attribute("new_owner", new_owner.to_string());

    Ok(Response::new().add_event(reject_proposal_event))
}

pub fn get_ownership_proposal(
    deps: Deps,
    proposal: Item<OwnerProposal>,
) -> StdResult<OwnerProposal> {
    let res = proposal.load(deps.storage)?;

    Ok(res)
}
