use crate::state::{Config, UserStake};

use cosmwasm_std::{coin, Uint128};
use margined_protocol::staking::{ExecuteMsg, QueryMsg, UserStakedResponse};
use margined_testing::staking_env::StakingEnv;
use osmosis_test_tube::{
    osmosis_std::types::cosmos::{bank::v1beta1::MsgSend, base::v1beta1::Coin},
    Account, Bank, Module, Wasm,
};

#[test]
fn test_stake_unstake_claim() {
    let env = StakingEnv::new();

    let bank = Bank::new(&env.app);
    let wasm = Wasm::new(&env.app);

    let (staking_address, collector_address) = env.deploy_staking_contracts(&wasm);

    // fund the fee collector
    {
        bank.send(
            MsgSend {
                from_address: env.signer.address(),
                to_address: collector_address,
                amount: [Coin {
                    amount: 1_000_000_000_000u128.to_string(),
                    denom: env.denoms["reward"].to_string(),
                }]
                .to_vec(),
            },
            &env.signer,
        )
        .unwrap();
    }

    wasm.execute(&staking_address, &ExecuteMsg::Unpause {}, &[], &env.signer)
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
            &env.signer,
        )
        .unwrap();

        let config: Config = wasm.query(&staking_address, &QueryMsg::Config {}).unwrap();
        assert_eq!(
            config.tokens_per_interval,
            Uint128::from(new_tokens_per_interval)
        );
    }

    // stake then increase time by one day
    {
        let amount_to_stake = 1_000_000_000u128; // 1,000@6dp esTOKEN
        wasm.execute(
            &staking_address,
            &ExecuteMsg::Stake {},
            &[coin(amount_to_stake, env.denoms["deposit"].to_string())],
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

        env.app.increase_time(24 * 60 * 60);

        let claimable: Uint128 = wasm
            .query(
                &staking_address,
                &QueryMsg::GetClaimable {
                    user: env.traders[0].address(),
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
            &[coin(amount_to_stake, env.denoms["deposit"].to_string())],
            &env.traders[1],
        )
        .unwrap();

        // check trader 0
        let stake: UserStake = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: env.traders[0].address(),
                },
            )
            .unwrap();
        assert_eq!(stake.staked_amounts, Uint128::from(1_000_000_000u128),);

        // check trader 1
        let stake: UserStake = wasm
            .query(
                &staking_address,
                &QueryMsg::GetUserStakedAmount {
                    user: env.traders[1].address(),
                },
            )
            .unwrap();
        assert_eq!(stake.staked_amounts, Uint128::from(500_000_000u128),);

        env.app.increase_time(24 * 60 * 60);

        // check claimable
        {
            let claimable: Uint128 = wasm
                .query(
                    &staking_address,
                    &QueryMsg::GetClaimable {
                        user: env.traders[0].address(),
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
                        user: env.traders[1].address(),
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
                &env.traders[0],
            );
            assert!(res.is_err());

            let amount_to_unstake = 500_000_001u128; // 500.000001@6dp stakedTOKEN
            let res = wasm.execute(
                &staking_address,
                &ExecuteMsg::Unstake {
                    amount: amount_to_unstake.into(),
                },
                &[],
                &env.traders[1],
            );
            assert!(res.is_err());
        }

        // unstake successfully and check user stake
        {
            assert_eq!(
                env.get_balance(env.traders[0].address(), env.denoms["deposit"].to_string()),
                Uint128::zero()
            );

            let amount_to_unstake = 1_000_000_000u128;
            wasm.execute(
                &staking_address,
                &ExecuteMsg::Unstake {
                    amount: amount_to_unstake.into(),
                },
                &[],
                &env.traders[0],
            )
            .unwrap();

            assert_eq!(
                env.get_balance(env.traders[0].address(), env.denoms["deposit"].to_string()),
                Uint128::from(amount_to_unstake)
            );

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
                &env.traders[0],
            );
            assert!(res.is_err());
        }

        // claim and check user balance
        {
            let user_balance: UserStakedResponse = wasm
                .query(
                    &staking_address,
                    &QueryMsg::GetUserStakedAmount {
                        user: env.traders[0].address(),
                    },
                )
                .unwrap();
            assert_eq!(user_balance.staked_amounts, Uint128::zero());

            wasm.execute(
                &staking_address,
                &ExecuteMsg::Claim {
                    recipient: Some(env.empty.address()),
                },
                &[],
                &env.traders[0],
            )
            .unwrap();

            assert_eq!(
                env.get_balance(env.empty.address(), env.denoms["reward"].to_string()),
                Uint128::from(2_976_501_000u128)
            );
        }

        env.app.increase_time(24 * 60 * 60);

        // check claimable
        {
            let claimable: Uint128 = wasm
                .query(
                    &staking_address,
                    &QueryMsg::GetClaimable {
                        user: env.traders[0].address(),
                    },
                )
                .unwrap();
            assert_eq!(claimable, Uint128::zero());

            let claimable: Uint128 = wasm
                .query(
                    &staking_address,
                    &QueryMsg::GetClaimable {
                        user: env.traders[1].address(),
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
