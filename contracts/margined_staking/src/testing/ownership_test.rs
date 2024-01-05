use cosmwasm_std::Addr;
use margined_protocol::staking::{ExecuteMsg, OwnerProposalResponse, QueryMsg};
use margined_utils::testing::test_tube::{TestTubeScenario, STAKING_CONTRACT_BYTES};
use osmosis_test_tube::{Account, Module, RunnerError, Wasm};

const PROPOSAL_DURATION: u64 = 1000;

#[test]
fn test_update_owner_staking() {
    let TestTubeScenario {
        router,
        accounts,
        usdc,
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

    // claim before a proposal is made
    {
        let err = wasm
            .execute(&staking, &ExecuteMsg::ClaimOwnership {}, &[], &env.signer)
            .unwrap_err();
        assert_eq!(
            err,
            RunnerError::ExecuteError {
                msg: "failed to execute message; message index: 0: Proposal not found: execute wasm contract failed".to_string()
            }
        );
    }

    // propose new owner
    wasm.execute(
        &staking,
        &ExecuteMsg::ProposeNewOwner {
            new_owner: env.traders[0].address(),
            duration: PROPOSAL_DURATION,
        },
        &[],
        &env.signer,
    )
    .unwrap();

    let owner: Addr = wasm.query(&staking, &QueryMsg::Owner {}).unwrap();
    assert_eq!(owner, env.signer.address());

    // reject claim by incorrect new owner
    {
        let err = wasm
            .execute(&staking, &ExecuteMsg::ClaimOwnership {}, &[], &env.signer)
            .unwrap_err();
        assert_eq!(
            err,
            RunnerError::ExecuteError {
                msg: "failed to execute message; message index: 0: Unauthorized: execute wasm contract failed".to_string()
            }
        );
    }

    // let proposal expire
    env.app.increase_time(PROPOSAL_DURATION + 1);

    // proposal fails due to expiry
    {
        let err = wasm
            .execute(
                &staking,
                &ExecuteMsg::ClaimOwnership {},
                &[],
                &env.traders[0],
            )
            .unwrap_err();
        assert_eq!(
            err,
            RunnerError::ExecuteError {
                msg: "failed to execute message; message index: 0: Expired: execute wasm contract failed".to_string()
            }
        );
    }

    let owner: Addr = wasm.query(&staking, &QueryMsg::Owner {}).unwrap();
    assert_eq!(owner, env.signer.address());

    // propose new owner
    wasm.execute(
        &staking,
        &ExecuteMsg::ProposeNewOwner {
            new_owner: env.traders[0].address(),
            duration: PROPOSAL_DURATION,
        },
        &[],
        &env.signer,
    )
    .unwrap();

    let owner: Addr = wasm.query(&staking, &QueryMsg::Owner {}).unwrap();
    assert_eq!(owner, env.signer.address());

    // proposal fails due to expiry
    {
        let err = wasm
            .execute(&staking, &ExecuteMsg::RejectOwner {}, &[], &env.traders[0])
            .unwrap_err();
        assert_eq!(
            err,
            RunnerError::ExecuteError {
                msg: "failed to execute message; message index: 0: Unauthorized: execute wasm contract failed".to_string()
            }
        );
    }

    // proposal fails due to expiry
    {
        wasm.execute(&staking, &ExecuteMsg::RejectOwner {}, &[], &env.signer)
            .unwrap();
    }

    // propose new owner
    wasm.execute(
        &staking,
        &ExecuteMsg::ProposeNewOwner {
            new_owner: env.traders[0].address(),
            duration: PROPOSAL_DURATION,
        },
        &[],
        &env.signer,
    )
    .unwrap();

    let block_time = env.app.get_block_time_seconds();

    let owner: Addr = wasm.query(&staking, &QueryMsg::Owner {}).unwrap();
    assert_eq!(owner, env.signer.address());

    // query ownership proposal
    {
        let proposal: OwnerProposalResponse = wasm
            .query(&staking, &QueryMsg::GetOwnershipProposal {})
            .unwrap();

        assert_eq!(proposal.owner, env.traders[0].address());
        assert_eq!(proposal.expiry, block_time as u64 + PROPOSAL_DURATION);
    }

    // claim ownership
    {
        wasm.execute(
            &staking,
            &ExecuteMsg::ClaimOwnership {},
            &[],
            &env.traders[0],
        )
        .unwrap();
    }

    let owner: Addr = wasm.query(&staking, &QueryMsg::Owner {}).unwrap();
    assert_eq!(owner, env.traders[0].address());
}
