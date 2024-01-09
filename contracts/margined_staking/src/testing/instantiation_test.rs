use crate::state::{Config, State};

use cosmwasm_std::{Addr, Timestamp};
use margined_common::asset::{AssetInfo, NATIVE_DENOM};
use margined_perp::margined_staking::{InstantiateMsg, QueryMsg};
use margined_utils::testing::test_tube::{TestTubeScenario, STAKING_CONTRACT_BYTES};
use osmosis_test_tube::{Account, Module, Wasm};

#[test]
fn test_instantiation() {
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

    let config: Config = wasm.query(&staking_address, &QueryMsg::Config {}).unwrap();
    assert_eq!(
        config,
        Config {
            fee_pool: fee_pool.addr(),
            deposit_token: AssetInfo::NativeToken {
                denom: NATIVE_DENOM.to_string(),
            },
            reward_token: AssetInfo::NativeToken {
                denom: NATIVE_DENOM.to_string(),
            },
            tokens_per_interval: 1_000_000u128.into(),
        }
    );

    let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
    assert_eq!(
        state,
        State {
            is_open: false,
            last_distribution: Timestamp::from_nanos(router.get_block_time_nanos() as u64),
        }
    );

    let owner: Addr = wasm.query(&staking_address, &QueryMsg::Owner {}).unwrap();
    assert_eq!(owner, Addr::unchecked(signer.address()));
}
