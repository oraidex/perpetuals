use crate::{
    error::ContractError,
    handle::{
        handle_claim, handle_pause, handle_stake, handle_unpause, handle_unstake,
        handle_update_config, handle_update_rewards,
    },
    query::{
        query_claimable, query_config, query_owner, query_state, query_total_staked_amount,
        query_user_staked_amount,
    },
    state::{
        Config, State, CONFIG, OWNER, OWNERSHIP_PROPOSAL, REWARDS_PER_TOKEN, STATE, TOTAL_STAKED,
    },
};

use crate::ownership::{
    get_ownership_proposal, handle_claim_ownership, handle_ownership_proposal,
    handle_ownership_proposal_rejection,
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    Uint128,
};
use cw2::set_contract_version;
use margined_perp::margined_staking::{ExecuteMsg, InstantiateMsg, QueryMsg};

pub const INSTANTIATE_REPLY_ID: u64 = 1u64;

// version info for migration info
pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(
        deps.storage,
        format!("crates.io:{CONTRACT_NAME}"),
        CONTRACT_VERSION,
    )?;

    CONFIG.save(
        deps.storage,
        &Config {
            fee_collector: deps.api.addr_validate(&msg.fee_collector)?,
            deposit_token: msg.deposit_token,
            reward_token: msg.reward_token,
            tokens_per_interval: msg.tokens_per_interval,
        },
    )?;

    STATE.save(
        deps.storage,
        &State {
            is_open: false,
            last_distribution: env.block.time,
        },
    )?;

    TOTAL_STAKED.save(deps.storage, &Uint128::zero())?;
    REWARDS_PER_TOKEN.save(deps.storage, &Uint128::zero())?;

    OWNER.set(deps, Some(info.sender))?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            tokens_per_interval,
        } => handle_update_config(deps, info, tokens_per_interval),
        ExecuteMsg::UpdateRewards {} => handle_update_rewards(deps, env),
        ExecuteMsg::Stake {} => handle_stake(deps, env, info),
        ExecuteMsg::Unstake { amount } => handle_unstake(deps, env, info, amount),
        ExecuteMsg::Claim { recipient } => handle_claim(deps, env, info, recipient),
        ExecuteMsg::Pause {} => handle_pause(deps, info),
        ExecuteMsg::Unpause {} => handle_unpause(deps, info),
        ExecuteMsg::ProposeNewOwner {
            new_owner,
            duration,
        } => handle_ownership_proposal(
            deps,
            info,
            env,
            new_owner,
            duration,
            OWNER,
            OWNERSHIP_PROPOSAL,
        ),
        ExecuteMsg::RejectOwner {} => {
            handle_ownership_proposal_rejection(deps, info, OWNER, OWNERSHIP_PROPOSAL)
        }
        ExecuteMsg::ClaimOwnership {} => {
            handle_claim_ownership(deps, info, env, OWNER, OWNERSHIP_PROPOSAL)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::Owner {} => {
            to_binary(&query_owner(deps).map_err(|err| StdError::generic_err(err.to_string()))?)
        }
        QueryMsg::GetClaimable { user } => to_binary(&query_claimable(deps, env, user)?),
        QueryMsg::GetUserStakedAmount { user } => to_binary(&query_user_staked_amount(deps, user)?),
        QueryMsg::GetTotalStakedAmount {} => to_binary(&query_total_staked_amount(deps)?),
        QueryMsg::GetOwnershipProposal {} => {
            to_binary(&get_ownership_proposal(deps, OWNERSHIP_PROPOSAL)?)
        }
    }
}
