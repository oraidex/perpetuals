use crate::state::{Config, State};

use cosmwasm_std::{Addr, Timestamp};
use margined_protocol::staking::QueryMsg;
use margined_testing::staking_env::StakingEnv;
use osmosis_test_tube::{Account, Module, Wasm};

const DEPOSIT_DENOM: &str = "umrg";
const REWARD_DENOM: &str = "uusdc";

#[test]
fn test_instantiation() {
    let env = StakingEnv::new();

    let wasm = Wasm::new(&env.app);

    let staking_address =
        env.deploy_staking_contract(&wasm, "margined-staking".to_string(), env.signer.address());

    let config: Config = wasm.query(&staking_address, &QueryMsg::Config {}).unwrap();
    assert_eq!(
        config,
        Config {
            fee_collector: Addr::unchecked(env.signer.address()),
            deposit_denom: DEPOSIT_DENOM.to_string(),
            deposit_decimals: 6u32,
            reward_denom: REWARD_DENOM.to_string(),
            reward_decimals: 6u32,
            tokens_per_interval: 1_000_000u128.into(),
        }
    );

    let state: State = wasm.query(&staking_address, &QueryMsg::State {}).unwrap();
    assert_eq!(
        state,
        State {
            is_open: false,
            last_distribution: Timestamp::from_nanos(env.app.get_block_time_nanos() as u64),
        }
    );

    let owner: Addr = wasm.query(&staking_address, &QueryMsg::Owner {}).unwrap();
    assert_eq!(owner, Addr::unchecked(env.signer.address()));
}
