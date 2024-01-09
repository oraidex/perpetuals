use std::str::FromStr;

use crate::state::{Config, UserStake};

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
fn test_stake_unstake_claim() {
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

    // fund the fee pool
    {
        bank.send(
            MsgSend {
                from_address: signer.address(),
                to_address: fee_pool.0.to_string(),
                amount: [Coin {
                    amount: 1_000_000_000_000u128.to_string(),
                    denom: NATIVE_DENOM.to_string(),
                }]
                .to_vec(),
            },
            &signer,
        )
        .unwrap();
    }

    wasm.execute(&staking_address, &ExecuteMsg::Unpause {}, &[], &signer)
        .unwrap();

    // update tokens per interval
    {
        let new_tokens_per_interval = 20_668u128; // 0.020668@6dp esTOKEN per second
        wasm.execute(
            &staking_address,
            &ExecuteMsg::UpdateConfig {
                tokens_per_interval: Some(new_tokens_per_interval.into()),
            },
            &[],
            &signer,
        )
        .unwrap();

        let config: Config = wasm.query(&staking_address, &QueryMsg::Config {}).unwrap();
        assert_eq!(
            config.tokens_per_interval,
            Uint128::from(new_tokens_per_interval)
        );
    }

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

    // stake then increase time by one day
    {
        let amount_to_stake = 1_000_000_000u128; // 1,000@6dp esTOKEN
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

        router.increase_time(24 * 60 * 60);

        let claimable: Uint128 = wasm
            .query(
                &staking_address,
                &QueryMsg::GetClaimable {
                    user: accounts[0].address(),
                },
            )
            .unwrap();
        assert_eq!(claimable, Uint128::from(1_785_715_000u128));
    }

    // stake then increase time by one day
    {
        let amount_to_stake = 500_000_000u128; // 500@6dp esTOKEN
        wasm.execute(
            &staking_address,
            &ExecuteMsg::Stake {},
            &[Coin {
                amount: amount_to_stake.to_string(),
                denom: NATIVE_DENOM.to_string(),
            }],
            &accounts[1],
        )
        .unwrap();

        // check trader 0
        let stake: UserStake = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: accounts[0].address(),
                },
            )
            .unwrap();
        assert_eq!(stake.staked_amounts, Uint128::from(1_000_000_000u128),);

        // check trader 1
        let stake: UserStake = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: accounts[1].address(),
                },
            )
            .unwrap();
        assert_eq!(stake.staked_amounts, Uint128::from(500_000_000u128),);

        router.increase_time(24 * 60 * 60);

        // check claimable
        {
            let claimable: Uint128 = wasm
                .query(
                    &staking_address,
                    &QueryMsg::GetClaimable {
                        user: accounts[0].address(),
                    },
                )
                .unwrap();
            assert_eq!(
                claimable,
                Uint128::from(1_785_715_000u128 + 1_190_579_000u128)
            );

            let claimable: Uint128 = wasm
                .query(
                    &staking_address,
                    &QueryMsg::GetClaimable {
                        user: accounts[1].address(),
                    },
                )
                .unwrap();
            assert_eq!(claimable, Uint128::from(595_238_000u128));
        }

        // unstake reverts
        {
            let amount_to_unstake = 1_000_000_001u128; // 1000.000001@6dp stakedTOKEN
            let res = wasm.execute(
                &staking_address,
                &ExecuteMsg::Unstake {
                    amount: amount_to_unstake.into(),
                },
                &[],
                &accounts[0],
            );
            assert!(res.is_err());

            let amount_to_unstake = 500_000_001u128; // 500.000001@6dp stakedTOKEN
            let res = wasm.execute(
                &staking_address,
                &ExecuteMsg::Unstake {
                    amount: amount_to_unstake.into(),
                },
                &[],
                &accounts[1],
            );
            assert!(res.is_err());
        }

        // unstake successfully and check user stake
        {
            assert_eq!(
                Uint128::from_str(
                    &bank
                        .query_balance(&QueryBalanceRequest {
                            address: accounts[1].address(),
                            denom: NATIVE_DENOM.to_string()
                        })
                        .unwrap()
                        .balance
                        .unwrap()
                        .amount
                )
                .unwrap(),
                Uint128::from(4998277445000u128)
            );

            let amount_to_unstake = 1_000_000_000u128;
            wasm.execute(
                &staking_address,
                &ExecuteMsg::Unstake {
                    amount: amount_to_unstake.into(),
                },
                &[],
                &accounts[0],
            )
            .unwrap();

            assert_eq!(
                Uint128::from_str(
                    &bank
                        .query_balance(&QueryBalanceRequest {
                            address: accounts[0].address(),
                            denom: NATIVE_DENOM.to_string()
                        })
                        .unwrap()
                        .balance
                        .unwrap()
                        .amount
                )
                .unwrap()
                    > Uint128::from(amount_to_unstake),
                true
            );

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
                    staked_amounts: Uint128::zero(),
                    previous_cumulative_rewards_per_token: Uint128::from(2_976_501u128),
                    claimable_rewards: Uint128::from(2_976_501_000u128),
                    cumulative_rewards: Uint128::from(2_976_501_000u128),
                }
            );
        }

        // unstake reverts
        {
            let amount_to_unstake = 1u128;
            let res = wasm.execute(
                &staking_address,
                &ExecuteMsg::Unstake {
                    amount: amount_to_unstake.into(),
                },
                &[],
                &accounts[0],
            );
            assert!(res.is_err());
        }

        // claim and check user balance
        {
            let user_balance: UserStakedResponse = wasm
                .query(
                    &staking_address,
                    &QueryMsg::GetUserStakedAmount {
                        user: accounts[0].address(),
                    },
                )
                .unwrap();
            assert_eq!(user_balance.staked_amounts, Uint128::zero());

            wasm.execute(
                &staking_address,
                &ExecuteMsg::Claim {
                    recipient: Some(accounts[2].address()),
                },
                &[],
                &accounts[0],
            )
            .unwrap();

            assert_eq!(
                Uint128::from_str(
                    &bank
                        .query_balance(&QueryBalanceRequest {
                            address: accounts[2].address(),
                            denom: NATIVE_DENOM.to_string()
                        })
                        .unwrap()
                        .balance
                        .unwrap()
                        .amount
                )
                .unwrap(),
                Uint128::from(5002540588500u128)
            );
        }

        router.increase_time(24 * 60 * 60);

        // check claimable
        {
            let claimable: Uint128 = wasm
                .query(
                    &staking_address,
                    &QueryMsg::GetClaimable {
                        user: accounts[0].address(),
                    },
                )
                .unwrap();
            assert_eq!(claimable, Uint128::zero());

            let claimable: Uint128 = wasm
                .query(
                    &staking_address,
                    &QueryMsg::GetClaimable {
                        user: accounts[1].address(),
                    },
                )
                .unwrap();
            assert_eq!(
                claimable,
                Uint128::from(595_238_000u128 + 1_786_025_000u128)
            );
        }
    }
}
