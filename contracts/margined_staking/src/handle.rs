use crate::{
    distributor::update_rewards,
    error::ContractError,
    helper::create_distribute_message_and_update_response,
    state::{UserStake, CONFIG, OWNER, STATE, TOTAL_STAKED, USER_STAKE},
};

use cosmwasm_std::{
    ensure, from_binary, Addr, DepsMut, Env, Event, MessageInfo, Response, StdResult, Uint128,
};
use cw20::Cw20ReceiveMsg;
use cw_utils::{must_pay, nonpayable};
use margined_common::asset::AssetInfo;
use margined_perp::margined_staking::Cw20HookMsg;

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Stake {}) => {
            let state = STATE.load(deps.storage)?;
            ensure!(state.is_open, ContractError::Paused {});
            let config = CONFIG.load(deps.storage)?;
            let contract_addr = match config.deposit_token {
                AssetInfo::Token { contract_addr } => contract_addr,
                _ => return Err(ContractError::NotCw20Token("deposit token".to_string())),
            };

            // check if the cw20 caller is deposit token
            if info.sender != contract_addr {
                return Err(ContractError::InvalidCw20);
            }
            let sender = deps.api.addr_validate(cw20_msg.sender.as_str())?;
            let sent_funds = cw20_msg.amount;

            _handle_stake(
                deps,
                env,
                sender,
                sent_funds,
                config.fee_pool,
                config.reward_token,
            )
        }

        Err(_) => Err(ContractError::InvalidCw20Hook {}),
    }
}

pub fn handle_update_config(
    deps: DepsMut,
    info: MessageInfo,
    tokens_per_interval: Option<Uint128>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    ensure!(
        OWNER.is_admin(deps.as_ref(), &info.sender)?,
        ContractError::Unauthorized {}
    );

    let event = Event::new("update_config");

    if let Some(tokens_per_interval) = tokens_per_interval {
        config.tokens_per_interval = tokens_per_interval;

        event
            .clone()
            .add_attribute("Tokens per interval", tokens_per_interval);
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_event(event))
}

pub fn handle_update_rewards(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let (_, rewards) = update_rewards(deps, env.clone(), env.contract.address.clone())?;

    let response = create_distribute_message_and_update_response(
        Response::new(),
        config.fee_pool,
        config.reward_token,
        rewards,
        env.contract.address.to_string(),
    )?;

    Ok(response.add_event(Event::new("update_rewards")))
}

pub fn handle_pause(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;

    ensure!(
        OWNER.is_admin(deps.as_ref(), &info.sender)?,
        ContractError::Unauthorized {}
    );

    if !state.is_open {
        return Err(ContractError::Paused {});
    }
    state.is_open = false;

    STATE.save(deps.storage, &state)?;

    Ok(Response::default().add_event(Event::new("paused")))
}

pub fn handle_unpause(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;

    ensure!(
        OWNER.is_admin(deps.as_ref(), &info.sender)?,
        ContractError::Unauthorized {}
    );

    if state.is_open {
        return Err(ContractError::NotPaused {});
    }

    state.is_open = true;

    STATE.save(deps.storage, &state)?;

    Ok(Response::default().add_event(Event::new("unpaused")))
}

pub fn handle_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    let sender = info.sender.clone();

    nonpayable(&info).map_err(|_| ContractError::InvalidFunds {})?;

    ensure!(state.is_open, ContractError::Paused {});

    let recipient = match recipient {
        Some(recipient) => {
            deps.api.addr_validate(recipient.as_str())?;
            recipient
        }
        None => sender.to_string(),
    };

    let (deps, rewards) = update_rewards(deps, env.clone(), sender.clone())?;

    let mut claimable_amount = Uint128::zero();
    USER_STAKE.update(deps.storage, sender.clone(), |res| -> StdResult<_> {
        let mut stake = match res {
            Some(stake) => stake,
            None => UserStake::default(),
        };

        claimable_amount = stake.claimable_rewards;
        stake.claimable_rewards = Uint128::zero();

        Ok(stake)
    })?;

    let mut response = create_distribute_message_and_update_response(
        Response::new(),
        config.fee_pool,
        config.reward_token.clone(),
        rewards,
        env.contract.address.to_string(),
    )?;

    if !claimable_amount.is_zero() {
        let msg_claim = config.reward_token.into_msg(
            recipient,
            claimable_amount,
            Some(env.contract.address.to_string()),
        )?;
        response = response.add_message(msg_claim);
    }

    Ok(response.add_event(Event::new("claim").add_attributes([
        ("amount", &claimable_amount.to_string()),
        ("user", &sender.to_string()),
    ])))
}

// this method is for native token, for cw20 token, need to write hook handle
pub fn handle_stake(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    ensure!(state.is_open, ContractError::Paused {});
    let config = CONFIG.load(deps.storage)?;
    let native_denom = match config.deposit_token {
        AssetInfo::NativeToken { denom } => denom,
        _ => return Err(ContractError::NotNativeToken("deposit token".to_string())),
    };

    let sent_funds: Uint128 =
        must_pay(&info, &native_denom).map_err(|_| ContractError::InvalidFunds {})?;

    let sender = info.sender;

    _handle_stake(
        deps,
        env,
        sender,
        sent_funds,
        config.fee_pool,
        config.reward_token,
    )
}

pub fn handle_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;

    ensure!(state.is_open, ContractError::Paused {});

    let sender = info.sender.clone();

    nonpayable(&info).map_err(|_| ContractError::InvalidFunds {})?;

    let (deps, rewards) = update_rewards(deps, env.clone(), sender.clone())?;

    USER_STAKE.update(deps.storage, sender.clone(), |res| -> StdResult<_> {
        let mut stake = match res {
            Some(stake) => stake,
            None => UserStake::default(),
        };

        stake.staked_amounts = stake.staked_amounts.checked_sub(amount)?;

        Ok(stake)
    })?;

    TOTAL_STAKED.update(deps.storage, |balance| -> StdResult<Uint128> {
        Ok(balance.checked_sub(amount)?)
    })?;

    let response = create_distribute_message_and_update_response(
        Response::new(),
        config.fee_pool,
        config.reward_token,
        rewards,
        env.contract.address.to_string(),
    )?;

    let msg_unstake = config.deposit_token.into_msg(
        sender.to_string(),
        amount,
        Some(env.contract.address.to_string()),
    )?;

    Ok(response
        .add_message(msg_unstake)
        .add_event(Event::new("unstake").add_attributes([
            ("amount", &amount.to_string()),
            ("user", &sender.to_string()),
        ])))
}

fn _handle_stake(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    sent_funds: Uint128,
    fee_pool: Addr,
    reward_token: AssetInfo,
) -> Result<Response, ContractError> {
    let (deps, rewards) = update_rewards(deps, env.clone(), sender.clone())?;

    USER_STAKE.update(deps.storage, sender.clone(), |res| -> StdResult<_> {
        let mut stake = match res {
            Some(stake) => stake,
            None => UserStake::default(),
        };

        stake.staked_amounts = stake.staked_amounts.checked_add(sent_funds)?;

        Ok(stake)
    })?;

    TOTAL_STAKED.update(deps.storage, |balance| -> StdResult<Uint128> {
        Ok(balance.checked_add(sent_funds)?)
    })?;

    let response = create_distribute_message_and_update_response(
        Response::new(),
        fee_pool,
        reward_token,
        rewards,
        env.contract.address.to_string(),
    )?;

    Ok(response.add_event(Event::new("stake").add_attributes([
        ("amount", &sent_funds.to_string()),
        ("user", &sender.to_string()),
    ])))
}
