use crate::state::{Config, State, UserStake};

use cosmwasm_std::{coin, Uint128};
use margined_protocol::staking::{ExecuteMsg, QueryMsg, UserStakedResponse};
use margined_testing::staking_env::StakingEnv;
use osmosis_test_tube::{
    osmosis_std::types::cosmos::{bank::v1beta1::MsgSend, base::v1beta1::Coin},
    Account, Bank, Module, Wasm,
};

const DEPOSIT_DENOM: &str = "umrg";

#[test]
fn test_unpause() {
    let env = StakingEnv::new();

    let wasm = Wasm::new(&env.app);

    let staking_address =
        env.deploy_staking_contract(&wasm, "margined-staking".to_string(), env.signer.address());

    let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
    assert!(!state.is_open);

    // cannot pause already paused
    {
        let err = wasm
            .execute(&staking_address, &ExecuteMsg::Pause {}, &[], &env.signer)
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Cannot perform action as contract is paused: execute wasm contract failed");
    }

    // cannot unpause if not owner
    {
        let err = wasm
            .execute(
                &staking_address,
                &ExecuteMsg::Unpause {},
                &[],
                &env.traders[0],
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Unauthorized: execute wasm contract failed");
    }

    // cannot stake if paused
    {
        let err = wasm
            .execute(
                &staking_address,
                &ExecuteMsg::Stake {},
                &[coin(1_000u128, env.denoms["deposit"].to_string())],
                &env.traders[0],
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Cannot perform action as contract is paused: execute wasm contract failed");
    }

    // cannot unstake if paused
    {
        let err = wasm
            .execute(
                &staking_address,
                &ExecuteMsg::Unstake {
                    amount: Uint128::zero(),
                },
                &[],
                &env.traders[0],
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Cannot perform action as contract is paused: execute wasm contract failed");
    }

    // cannot claim if paused
    {
        let err = wasm
            .execute(
                &staking_address,
                &ExecuteMsg::Claim { recipient: None },
                &[],
                &env.traders[0],
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Cannot perform action as contract is paused: execute wasm contract failed");
    }

    // should be able to unpause if owner
    {
        wasm.execute(&staking_address, &ExecuteMsg::Unpause {}, &[], &env.signer)
            .unwrap();
    }

    let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
    assert!(state.is_open);
}

#[test]
fn test_pause() {
    let env = StakingEnv::new();

    let wasm = Wasm::new(&env.app);

    let staking_address =
        env.deploy_staking_contract(&wasm, "margined-staking".to_string(), env.signer.address());

    // should be able to unpause if owner
    {
        wasm.execute(&staking_address, &ExecuteMsg::Unpause {}, &[], &env.signer)
            .unwrap();
    }

    let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
    assert!(state.is_open);

    // cannot pause if not owner
    {
        let err = wasm
            .execute(
                &staking_address,
                &ExecuteMsg::Pause {},
                &[],
                &env.traders[0],
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Unauthorized: execute wasm contract failed");
    }

    // should be able to pause if owner
    {
        wasm.execute(&staking_address, &ExecuteMsg::Pause {}, &[], &env.signer)
            .unwrap();
    }

    let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
    assert!(!state.is_open);
}

#[test]
fn test_update_config() {
    let env = StakingEnv::new();

    let wasm = Wasm::new(&env.app);

    let staking_address =
        env.deploy_staking_contract(&wasm, "margined-staking".to_string(), env.signer.address());
    let config_before: Config = wasm.query(&staking_address, &QueryMsg::Config {}).unwrap();

    // should update config if owner
    {
        wasm.execute(
            &staking_address,
            &ExecuteMsg::UpdateConfig {
                tokens_per_interval: Some(128u128.into()),
            },
            &[],
            &env.signer,
        )
        .unwrap();

        let config_after: Config = wasm.query(&staking_address, &QueryMsg::Config {}).unwrap();
        assert_eq!(Uint128::from(128u128), config_after.tokens_per_interval);
        assert_ne!(
            config_before.tokens_per_interval,
            config_after.tokens_per_interval,
        );
    }

    // returns error if not owner
    {
        wasm.execute(
            &staking_address,
            &ExecuteMsg::UpdateConfig {
                tokens_per_interval: Some(128u128.into()),
            },
            &[],
            &env.traders[0],
        )
        .unwrap_err();
    }
}

#[test]
fn test_staking() {
    let env = StakingEnv::new();

    let bank = Bank::new(&env.app);
    let wasm = Wasm::new(&env.app);

    let (staking_address, collector_address) = env.deploy_staking_contracts(&wasm);

    bank.send(
        MsgSend {
            from_address: env.signer.address(),
            to_address: collector_address,
            amount: [Coin {
                amount: 1_000_000_000u128.to_string(),
                denom: env.denoms["reward"].to_string(),
            }]
            .to_vec(),
        },
        &env.signer,
    )
    .unwrap();

    wasm.execute(&staking_address, &ExecuteMsg::Unpause {}, &[], &env.signer)
        .unwrap();

    // returns error with wrong asset
    {
        let amount_to_stake = 1_000_000u128;
        let err = wasm
            .execute(
                &staking_address,
                &ExecuteMsg::Stake {},
                &[coin(amount_to_stake, env.denoms["reward"].to_string())],
                &env.traders[0],
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Invalid funds: execute wasm contract failed");
    }

    // returns error with insufficient funds
    {
        let amount_to_stake = 1_000_000_000_000u128;
        let err = wasm
            .execute(
                &staking_address,
                &ExecuteMsg::Stake {},
                &[coin(amount_to_stake, env.denoms["deposit"].to_string())],
                &env.traders[0],
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: 1000000000umrg is smaller than 1000000000000umrg: insufficient funds");
    }

    // should be able to stake
    {
        let balance_before =
            env.get_balance(env.traders[0].address(), env.denoms["deposit"].to_string());

        let amount_to_stake = 1_000_000u128;
        wasm.execute(
            &staking_address,
            &ExecuteMsg::Stake {},
            &[coin(amount_to_stake, DEPOSIT_DENOM)],
            &env.traders[0],
        )
        .unwrap();

        let stake: UserStake = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: env.traders[0].address(),
                },
            )
            .unwrap();
        assert_eq!(
            stake,
            UserStake {
                staked_amounts: amount_to_stake.into(),
                previous_cumulative_rewards_per_token: Uint128::zero(),
                claimable_rewards: Uint128::zero(),
                cumulative_rewards: Uint128::zero(),
            }
        );

        let balance_after =
            env.get_balance(env.traders[0].address(), env.denoms["deposit"].to_string());
        let staked_balance: UserStakedResponse = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: env.traders[0].address(),
                },
            )
            .unwrap();

        assert_eq!(
            balance_before - Uint128::from(amount_to_stake),
            balance_after
        );
        assert_eq!(
            staked_balance.staked_amounts,
            Uint128::from(amount_to_stake)
        );
    }

    // account should be default before staking
    {
        let stake: UserStake = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: env.traders[1].address(),
                },
            )
            .unwrap();
        assert_eq!(stake, UserStake::default());
    }
}

#[test]
fn test_unstaking() {
    let env = StakingEnv::new();

    let wasm = Wasm::new(&env.app);

    let (staking_address, _) = env.deploy_staking_contracts(&wasm);

    wasm.execute(&staking_address, &ExecuteMsg::Unpause {}, &[], &env.signer)
        .unwrap();

    let amount_to_stake = 1_000_000u128;
    wasm.execute(
        &staking_address,
        &ExecuteMsg::Stake {},
        &[coin(amount_to_stake, DEPOSIT_DENOM)],
        &env.traders[0],
    )
    .unwrap();

    // returns error if tokens are sent
    {
        let amount_to_stake = 1_000u128;
        let err = wasm
            .execute(
                &staking_address,
                &ExecuteMsg::Unstake {
                    amount: amount_to_stake.into(),
                },
                &[coin(amount_to_stake, DEPOSIT_DENOM)],
                &env.traders[0],
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Invalid funds: execute wasm contract failed");
    }

    // should unstake half
    {
        let balance_before =
            env.get_balance(env.traders[0].address(), env.denoms["deposit"].to_string());
        let balance_before_staked: UserStakedResponse = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: env.traders[0].address(),
                },
            )
            .unwrap();

        let amount_to_unstake = 500_000u128;
        wasm.execute(
            &staking_address,
            &ExecuteMsg::Unstake {
                amount: amount_to_unstake.into(),
            },
            &[],
            &env.traders[0],
        )
        .unwrap();

        let balance_after =
            env.get_balance(env.traders[0].address(), env.denoms["deposit"].to_string());
        let balance_after_staked: UserStakedResponse = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: env.traders[0].address(),
                },
            )
            .unwrap();

        assert_eq!(
            balance_before + Uint128::from(amount_to_unstake),
            balance_after
        );
        assert_eq!(
            balance_before_staked.staked_amounts - Uint128::from(amount_to_unstake),
            balance_after_staked.staked_amounts
        );
    }
}

#[test]
fn test_claim() {
    let env = StakingEnv::new();

    let wasm = Wasm::new(&env.app);
    let bank = Bank::new(&env.app);

    let (staking_address, collector_address) = env.deploy_staking_contracts(&wasm);

    bank.send(
        MsgSend {
            from_address: env.signer.address(),
            to_address: collector_address,
            amount: [Coin {
                amount: 1_000_000_000u128.to_string(),
                denom: env.denoms["reward"].to_string(),
            }]
            .to_vec(),
        },
        &env.signer,
    )
    .unwrap();

    wasm.execute(&staking_address, &ExecuteMsg::Unpause {}, &[], &env.signer)
        .unwrap();

    let amount_to_stake = 1_000_000u128;
    wasm.execute(
        &staking_address,
        &ExecuteMsg::Stake {},
        &[coin(amount_to_stake, DEPOSIT_DENOM)],
        &env.traders[0],
    )
    .unwrap();

    // should all be zero staking
    {
        let stake: UserStake = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: env.traders[0].address(),
                },
            )
            .unwrap();
        assert_eq!(
            stake,
            UserStake {
                staked_amounts: amount_to_stake.into(),
                previous_cumulative_rewards_per_token: Uint128::zero(),
                claimable_rewards: Uint128::zero(),
                cumulative_rewards: Uint128::zero(),
            }
        );
    }

    // returns error if tokens are sent
    {
        let amount = 1_000u128;
        let err = wasm
            .execute(
                &staking_address,
                &ExecuteMsg::Claim { recipient: None },
                &[coin(amount, DEPOSIT_DENOM)],
                &env.traders[0],
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Invalid funds: execute wasm contract failed");
    }

    env.app.increase_time(90u64);

    // should update distribution time
    {
        let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
        let previous_distribution_time = state.last_distribution;

        wasm.execute(
            &staking_address,
            &ExecuteMsg::UpdateRewards {},
            &[],
            &env.traders[1],
        )
        .unwrap();

        let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
        let distribution_time = state.last_distribution;

        assert_eq!(
            distribution_time.seconds() - previous_distribution_time.seconds(),
            100u64
        );

        // 100 seconds passed, 1 reward per second, 1_000_000 staked
        // 100 * 1_000_000 *
        let expected_claimable = Uint128::from(100_000_000u128);
        let claimable_amount: Uint128 = wasm
            .query(
                &staking_address,
                &QueryMsg::GetClaimable {
                    user: env.traders[0].address(),
                },
            )
            .unwrap();
        assert_eq!(claimable_amount, expected_claimable);

        let stake: UserStake = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: env.traders[0].address(),
                },
            )
            .unwrap();
        assert_eq!(
            stake,
            UserStake {
                staked_amounts: amount_to_stake.into(),
                previous_cumulative_rewards_per_token: Uint128::zero(),
                claimable_rewards: Uint128::zero(),
                cumulative_rewards: Uint128::zero(),
            }
        );
    }

    // does nothing except consume gas if user has nothing to claim
    {
        env.app.increase_time(1u64);
        let balance_before =
            env.get_balance(env.traders[1].address(), env.denoms["reward"].to_string());
        wasm.execute(
            &staking_address,
            &ExecuteMsg::Claim { recipient: None },
            &[],
            &env.traders[1],
        )
        .unwrap();

        let balance_after =
            env.get_balance(env.traders[1].address(), env.denoms["reward"].to_string());
        assert_eq!(balance_before, balance_after);
    }

    // should claim all rewards
    {
        env.app.increase_time(1u64);
        let balance_before =
            env.get_balance(env.traders[0].address(), env.denoms["reward"].to_string());
        let expected_claimable = Uint128::from(112_000_000u128);

        wasm.execute(
            &staking_address,
            &ExecuteMsg::Claim { recipient: None },
            &[],
            &env.traders[0],
        )
        .unwrap();

        let balance_after =
            env.get_balance(env.traders[0].address(), env.denoms["reward"].to_string());

        assert_eq!(balance_before + expected_claimable, balance_after);
    }
}
