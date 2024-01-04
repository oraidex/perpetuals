use crate::{
    query::query_pending_rewards,
    state::{CONFIG, REWARDS_PER_TOKEN, STATE, TOTAL_STAKED, USER_STAKE},
};

use cosmwasm_std::{Addr, Deps, DepsMut, Env, StdResult, Storage, Uint128};

pub fn calculate_rewards(deps: Deps, env: Env) -> StdResult<Uint128> {
    let config = CONFIG.load(deps.storage)?;

    let block_rewards = query_pending_rewards(deps, env)?;

    let balance = config
        .reward_token
        .query_balance(&deps.querier, config.fee_pool)?;

    Ok(block_rewards.min(balance))
}

pub fn update_distribution_time(storage: &mut dyn Storage, env: Env) -> StdResult<()> {
    STATE.update(storage, |mut s| -> StdResult<_> {
        s.last_distribution = env.block.time;
        Ok(s)
    })?;

    Ok(())
}

pub fn update_rewards(deps: DepsMut, env: Env, account: Addr) -> StdResult<(DepsMut, Uint128)> {
    let config = CONFIG.load(deps.storage)?;
    let decimal_places = 10u128.pow(config.reward_token.get_decimals(&deps.querier)? as u32);
    // default is zero
    let block_rewards = calculate_rewards(deps.as_ref(), env.clone()).unwrap_or_default();
    update_distribution_time(deps.storage, env.clone())?;

    if block_rewards.is_zero() {
        return Ok((deps, block_rewards));
    }

    let supply = TOTAL_STAKED.load(deps.storage)?;

    let mut cumulative_rewards = REWARDS_PER_TOKEN.load(deps.storage)?;
    if !supply.is_zero() && !block_rewards.is_zero() {
        cumulative_rewards = cumulative_rewards.checked_add(
            block_rewards
                .checked_mul(decimal_places.into())?
                .checked_div(supply)?,
        )?;
        REWARDS_PER_TOKEN.save(deps.storage, &cumulative_rewards)?;
    }

    if account == env.contract.address {
        return Ok((deps, block_rewards));
    }

    let mut user = USER_STAKE
        .load(deps.storage, account.clone())
        .unwrap_or_default();

    let delta_rewards =
        cumulative_rewards.checked_sub(user.previous_cumulative_rewards_per_token)?;

    let account_reward = user
        .staked_amounts
        .checked_mul(delta_rewards)?
        .checked_div(decimal_places.into())?;

    user.claimable_rewards = user.claimable_rewards.checked_add(account_reward)?;
    user.previous_cumulative_rewards_per_token = cumulative_rewards;

    if !user.claimable_rewards.is_zero() && !user.staked_amounts.is_zero() {
        let next_cumulative_reward = user
            .cumulative_rewards
            .checked_add(user.claimable_rewards)?;

        user.cumulative_rewards = next_cumulative_reward;
    }

    USER_STAKE.save(deps.storage, account, &user)?;

    Ok((deps, block_rewards))
}
