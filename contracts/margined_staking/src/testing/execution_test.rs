use std::str::FromStr;

use crate::state::{Config, State, UserStake};

use cosmwasm_std::Uint128;
use margined_common::asset::{AssetInfo, NATIVE_DENOM};

use margined_perp::margined_staking::{ExecuteMsg, InstantiateMsg, QueryMsg, UserStakedResponse};
use margined_utils::testing::test_tube::{TestTubeScenario, STAKING_CONTRACT_BYTES};
use osmosis_test_tube::{
    cosmrs::proto::cosmos::{
        bank::v1beta1::{MsgSend, QueryBalanceRequest},
        base::v1beta1::Coin,
    },
    Account, Bank, Module, Wasm,
};

#[test]
fn test_unpause() {
    let TestTubeScenario {
        router,
        accounts,
        fee_pool,
        ..
    } = TestTubeScenario::default();

    let signer = &accounts[0];

    let wasm = Wasm::new(&router);

    let staking_code_id = wasm
        .store_code(STAKING_CONTRACT_BYTES, None, signer)
        .unwrap()
        .data
        .code_id;

    let staking_address = wasm
        .instantiate(
            staking_code_id,
            &InstantiateMsg {
                fee_pool: fee_pool.addr().to_string(),
                deposit_token: AssetInfo::NativeToken {
                    denom: NATIVE_DENOM.to_string(),
                },
                reward_token: AssetInfo::NativeToken {
                    denom: NATIVE_DENOM.to_string(),
                },
                // deposit_token: AssetInfo::Token {
                //     contract_addr: usdc.addr(),
                // },
                // reward_token: AssetInfo::Token {
                //     contract_addr: usdc.addr(),
                // }, // should be ORAIX
                tokens_per_interval: 1_000_000u128.into(),
            },
            None,
            Some("margined-staking"),
            &[],
            signer,
        )
        .unwrap()
        .data
        .address;

    let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
    assert!(!state.is_open);

    // cannot pause already paused
    {
        let err = wasm
            .execute(&staking_address, &ExecuteMsg::Pause {}, &[], signer)
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Cannot perform action as contract is paused: execute wasm contract failed");
    }

    // cannot unpause if not owner
    {
        let err = wasm
            .execute(&staking_address, &ExecuteMsg::Unpause {}, &[], &accounts[1])
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Unauthorized: execute wasm contract failed");
    }

    // cannot stake if paused
    {
        let err = wasm
            .execute(
                &staking_address,
                &ExecuteMsg::Stake {},
                &[Coin {
                    amount: "1_000".to_string(),
                    denom: NATIVE_DENOM.to_string(),
                }],
                signer,
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
                signer,
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
                signer,
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Cannot perform action as contract is paused: execute wasm contract failed");
    }

    // should be able to unpause if owner
    {
        wasm.execute(&staking_address, &ExecuteMsg::Unpause {}, &[], signer)
            .unwrap();
    }

    let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
    assert!(state.is_open);
}

#[test]
fn test_pause() {
    let TestTubeScenario {
        router,
        accounts,
        fee_pool,
        ..
    } = TestTubeScenario::default();

    let signer = &accounts[0];

    let wasm = Wasm::new(&router);

    let staking_code_id = wasm
        .store_code(STAKING_CONTRACT_BYTES, None, signer)
        .unwrap()
        .data
        .code_id;

    let staking_address = wasm
        .instantiate(
            staking_code_id,
            &InstantiateMsg {
                fee_pool: fee_pool.addr().to_string(),
                deposit_token: AssetInfo::NativeToken {
                    denom: NATIVE_DENOM.to_string(),
                },
                reward_token: AssetInfo::NativeToken {
                    denom: NATIVE_DENOM.to_string(),
                },
                // deposit_token: AssetInfo::Token {
                //     contract_addr: usdc.addr(),
                // },
                // reward_token: AssetInfo::Token {
                //     contract_addr: usdc.addr(),
                // }, // should be ORAIX
                tokens_per_interval: 1_000_000u128.into(),
            },
            None,
            Some("margined-staking"),
            &[],
            signer,
        )
        .unwrap()
        .data
        .address;

    // should be able to unpause if owner
    {
        wasm.execute(&staking_address, &ExecuteMsg::Unpause {}, &[], &signer)
            .unwrap();
    }

    let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
    assert!(state.is_open);

    // cannot pause if not owner
    {
        let err = wasm
            .execute(&staking_address, &ExecuteMsg::Pause {}, &[], &accounts[1])
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Unauthorized: execute wasm contract failed");
    }

    // should be able to pause if owner
    {
        wasm.execute(&staking_address, &ExecuteMsg::Pause {}, &[], &signer)
            .unwrap();
    }

    let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
    assert!(!state.is_open);
}

#[test]
fn test_update_config() {
    let TestTubeScenario {
        router,
        accounts,
        fee_pool,
        ..
    } = TestTubeScenario::default();

    let signer = &accounts[0];

    let wasm = Wasm::new(&router);

    let staking_code_id = wasm
        .store_code(STAKING_CONTRACT_BYTES, None, signer)
        .unwrap()
        .data
        .code_id;

    let staking_address = wasm
        .instantiate(
            staking_code_id,
            &InstantiateMsg {
                fee_pool: fee_pool.addr().to_string(),
                deposit_token: AssetInfo::NativeToken {
                    denom: NATIVE_DENOM.to_string(),
                },
                reward_token: AssetInfo::NativeToken {
                    denom: NATIVE_DENOM.to_string(),
                },
                // deposit_token: AssetInfo::Token {
                //     contract_addr: usdc.addr(),
                // },
                // reward_token: AssetInfo::Token {
                //     contract_addr: usdc.addr(),
                // }, // should be ORAIX
                tokens_per_interval: 1_000_000u128.into(),
            },
            None,
            Some("margined-staking"),
            &[],
            signer,
        )
        .unwrap()
        .data
        .address;
    let config_before: Config = wasm.query(&staking_address, &QueryMsg::Config {}).unwrap();

    // should update config if owner
    {
        wasm.execute(
            &staking_address,
            &ExecuteMsg::UpdateConfig {
                tokens_per_interval: Some(128u128.into()),
            },
            &[],
            &signer,
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
            &accounts[1],
        )
        .unwrap_err();
    }
}

#[test]
fn test_staking() {
    let TestTubeScenario {
        router,
        accounts,
        fee_pool,
        ..
    } = TestTubeScenario::default();

    let signer = &accounts[0];

    let wasm = Wasm::new(&router);

    let staking_code_id = wasm
        .store_code(STAKING_CONTRACT_BYTES, None, signer)
        .unwrap()
        .data
        .code_id;

    let staking_address = wasm
        .instantiate(
            staking_code_id,
            &InstantiateMsg {
                fee_pool: fee_pool.addr().to_string(),
                deposit_token: AssetInfo::NativeToken {
                    denom: NATIVE_DENOM.to_string(),
                },
                reward_token: AssetInfo::NativeToken {
                    denom: NATIVE_DENOM.to_string(),
                },
                // deposit_token: AssetInfo::Token {
                //     contract_addr: usdc.addr(),
                // },
                // reward_token: AssetInfo::Token {
                //     contract_addr: usdc.addr(),
                // }, // should be ORAIX
                tokens_per_interval: 1_000_000u128.into(),
            },
            None,
            Some("margined-staking"),
            &[],
            signer,
        )
        .unwrap()
        .data
        .address;

    let bank = Bank::new(&router);

    bank.send(
        MsgSend {
            from_address: signer.address(),
            to_address: fee_pool.0.to_string(),
            amount: [Coin {
                amount: 1_000_000_000u128.to_string(),
                denom: NATIVE_DENOM.to_string(),
            }]
            .to_vec(),
        },
        &signer,
    )
    .unwrap();

    wasm.execute(&staking_address, &ExecuteMsg::Unpause {}, &[], &signer)
        .unwrap();

    let _res = wasm
        .execute(
            fee_pool.0.as_str(),
            &margined_perp::margined_fee_pool::ExecuteMsg::AddToken {
                token: NATIVE_DENOM.to_string(),
            },
            &[],
            &signer,
        )
        .unwrap();

    // change owner of fee pool to staking contract
    let _res = wasm
        .execute(
            fee_pool.0.as_str(),
            &margined_perp::margined_fee_pool::ExecuteMsg::UpdateOwner {
                owner: staking_address.clone(),
            },
            &[],
            &signer,
        )
        .unwrap();

    // returns error with wrong asset
    {
        let err = wasm
            .execute(&staking_address, &ExecuteMsg::Stake {}, &[], &accounts[0])
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Invalid funds: execute wasm contract failed");
    }

    // should be able to stake
    {
        let balance_before = Uint128::from_str(
            &bank
                .query_balance(&QueryBalanceRequest {
                    address: accounts[0].address(),
                    denom: NATIVE_DENOM.to_string(),
                })
                .unwrap()
                .balance
                .unwrap_or_default()
                .amount,
        )
        .unwrap();

        let amount_to_stake = 1_000_000u128;
        let _res = wasm
            .execute(
                &staking_address,
                &ExecuteMsg::Stake {},
                &[Coin {
                    amount: amount_to_stake.to_string(),
                    denom: NATIVE_DENOM.to_string(),
                }],
                &accounts[0],
            )
            .unwrap();

        let stake: UserStake = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: accounts[0].address(),
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

        let balance_after = Uint128::from_str(
            &bank
                .query_balance(&QueryBalanceRequest {
                    address: accounts[0].address(),
                    denom: NATIVE_DENOM.to_string(),
                })
                .unwrap()
                .balance
                .unwrap_or_default()
                .amount,
        )
        .unwrap();

        let staked_balance: UserStakedResponse = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: accounts[0].address(),
                },
            )
            .unwrap();

        // due to gas_used
        assert_eq!(
            balance_before - Uint128::from(amount_to_stake) > balance_after,
            true
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
                    user: accounts[1].address(),
                },
            )
            .unwrap();
        assert_eq!(stake, UserStake::default());
    }
}

#[test]
fn test_unstaking() {
    let TestTubeScenario {
        router,
        accounts,
        fee_pool,
        ..
    } = TestTubeScenario::default();

    let signer = &accounts[0];

    let wasm = Wasm::new(&router);

    let staking_code_id = wasm
        .store_code(STAKING_CONTRACT_BYTES, None, signer)
        .unwrap()
        .data
        .code_id;

    let staking_address = wasm
        .instantiate(
            staking_code_id,
            &InstantiateMsg {
                fee_pool: fee_pool.addr().to_string(),
                deposit_token: AssetInfo::NativeToken {
                    denom: NATIVE_DENOM.to_string(),
                },
                reward_token: AssetInfo::NativeToken {
                    denom: NATIVE_DENOM.to_string(),
                },
                // deposit_token: AssetInfo::Token {
                //     contract_addr: usdc.addr(),
                // },
                // reward_token: AssetInfo::Token {
                //     contract_addr: usdc.addr(),
                // }, // should be ORAIX
                tokens_per_interval: 1_000_000u128.into(),
            },
            None,
            Some("margined-staking"),
            &[],
            signer,
        )
        .unwrap()
        .data
        .address;

    wasm.execute(&staking_address, &ExecuteMsg::Unpause {}, &[], &signer)
        .unwrap();

    let amount_to_stake = 1_000_000u128;
    wasm.execute(
        &staking_address,
        &ExecuteMsg::Stake {},
        &[Coin {
            amount: amount_to_stake.to_string(),
            denom: NATIVE_DENOM.to_string(),
        }],
        &accounts[0],
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
                &[Coin {
                    amount: amount_to_stake.to_string(),
                    denom: NATIVE_DENOM.to_string(),
                }],
                &accounts[0],
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Invalid funds: execute wasm contract failed");
    }

    let bank = Bank::new(&router);
    // should unstake half
    {
        let balance_before = bank
            .query_balance(&QueryBalanceRequest {
                address: accounts[0].address(),
                denom: NATIVE_DENOM.to_string(),
            })
            .unwrap()
            .balance
            .unwrap();

        let balance_before_staked: UserStakedResponse = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: accounts[0].address(),
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
            &accounts[0],
        )
        .unwrap();

        let balance_after = bank
            .query_balance(&QueryBalanceRequest {
                address: accounts[0].address(),
                denom: NATIVE_DENOM.to_string(),
            })
            .unwrap()
            .balance
            .unwrap();

        let balance_after_staked: UserStakedResponse = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: accounts[0].address(),
                },
            )
            .unwrap();

        assert_eq!(
            Uint128::from_str(&balance_before.amount).unwrap() + Uint128::from(amount_to_unstake)
                > Uint128::from_str(&balance_after.amount).unwrap(),
            true
        );
        assert_eq!(
            balance_before_staked.staked_amounts - Uint128::from(amount_to_unstake),
            balance_after_staked.staked_amounts
        );
    }
}

#[test]
fn test_claim() {
    let TestTubeScenario {
        router,
        accounts,
        fee_pool,
        ..
    } = TestTubeScenario::default();

    let signer = &accounts[0];

    let wasm = Wasm::new(&router);

    let staking_code_id = wasm
        .store_code(STAKING_CONTRACT_BYTES, None, signer)
        .unwrap()
        .data
        .code_id;

    let staking_address = wasm
        .instantiate(
            staking_code_id,
            &InstantiateMsg {
                fee_pool: fee_pool.addr().to_string(),
                deposit_token: AssetInfo::NativeToken {
                    denom: NATIVE_DENOM.to_string(),
                },
                reward_token: AssetInfo::NativeToken {
                    denom: NATIVE_DENOM.to_string(),
                },
                // deposit_token: AssetInfo::Token {
                //     contract_addr: usdc.addr(),
                // },
                // reward_token: AssetInfo::Token {
                //     contract_addr: usdc.addr(),
                // }, // should be ORAIX
                tokens_per_interval: 1_000_000u128.into(),
            },
            None,
            Some("margined-staking"),
            &[],
            signer,
        )
        .unwrap()
        .data
        .address;
    let bank = Bank::new(&router);

    bank.send(
        MsgSend {
            from_address: signer.address(),
            to_address: fee_pool.0.to_string(),
            amount: [Coin {
                amount: 1_000_000_000u128.to_string(),
                denom: NATIVE_DENOM.to_string(),
            }]
            .to_vec(),
        },
        &signer,
    )
    .unwrap();

    wasm.execute(&staking_address, &ExecuteMsg::Unpause {}, &[], &signer)
        .unwrap();

    let _res = wasm
        .execute(
            fee_pool.0.as_str(),
            &margined_perp::margined_fee_pool::ExecuteMsg::AddToken {
                token: NATIVE_DENOM.to_string(),
            },
            &[],
            &signer,
        )
        .unwrap();

    // change owner of fee pool to staking contract
    let _res = wasm
        .execute(
            fee_pool.0.as_str(),
            &margined_perp::margined_fee_pool::ExecuteMsg::UpdateOwner {
                owner: staking_address.clone(),
            },
            &[],
            &signer,
        )
        .unwrap();

    let amount_to_stake = 1_000_000u128;
    wasm.execute(
        &staking_address,
        &ExecuteMsg::Stake {},
        &[Coin {
            amount: amount_to_stake.to_string(),
            denom: NATIVE_DENOM.to_string(),
        }],
        &accounts[0],
    )
    .unwrap();

    // should all be zero staking
    {
        let stake: UserStake = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: accounts[0].address(),
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
                &[Coin {
                    amount: amount.to_string(),
                    denom: NATIVE_DENOM.to_string(),
                }],
                &accounts[0],
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "execute error: failed to execute message; message index: 0: Invalid funds: execute wasm contract failed");
    }

    router.increase_time(90u64);

    // should update distribution time
    {
        let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
        let previous_distribution_time = state.last_distribution;

        wasm.execute(
            &staking_address,
            &ExecuteMsg::UpdateRewards {},
            &[],
            &accounts[1],
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
                    user: accounts[0].address(),
                },
            )
            .unwrap();
        assert_eq!(claimable_amount, expected_claimable);

        let stake: UserStake = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: accounts[0].address(),
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

    let bank = Bank::new(&router);

    // does nothing except consume gas if user has nothing to claim
    {
        router.increase_time(1u64);
        let balance_before = bank
            .query_balance(&QueryBalanceRequest {
                address: accounts[1].address(),
                denom: NATIVE_DENOM.to_string(),
            })
            .unwrap()
            .balance
            .unwrap();

        wasm.execute(
            &staking_address,
            &ExecuteMsg::Claim { recipient: None },
            &[],
            &accounts[1],
        )
        .unwrap();

        let balance_after = bank
            .query_balance(&QueryBalanceRequest {
                address: accounts[1].address(),
                denom: NATIVE_DENOM.to_string(),
            })
            .unwrap()
            .balance
            .unwrap();

        // minus gas_used
        assert_eq!(
            Uint128::from_str(&balance_before.amount).unwrap()
                > Uint128::from_str(&balance_after.amount).unwrap(),
            true
        );
    }

    // should claim all rewards
    {
        router.increase_time(1u64);
        let balance_before = bank
            .query_balance(&QueryBalanceRequest {
                address: accounts[0].address(),
                denom: NATIVE_DENOM.to_string(),
            })
            .unwrap()
            .balance
            .unwrap();
        let expected_claimable = Uint128::from(112_000_000u128);

        wasm.execute(
            &staking_address,
            &ExecuteMsg::Claim { recipient: None },
            &[],
            &accounts[0],
        )
        .unwrap();

        let balance_after = bank
            .query_balance(&QueryBalanceRequest {
                address: accounts[0].address(),
                denom: NATIVE_DENOM.to_string(),
            })
            .unwrap()
            .balance
            .unwrap();

        // minus gas_used
        assert_eq!(
            Uint128::from_str(&balance_before.amount).unwrap() + expected_claimable
                > Uint128::from_str(&balance_after.amount).unwrap(),
            true
        );
    }
}
