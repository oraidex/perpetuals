use cosmwasm_std::Addr;
use margined_protocol::staking::{ExecuteMsg, OwnerProposalResponse, QueryMsg};
use margined_testing::staking_env::StakingEnv;
use osmosis_test_tube::{Account, Module, RunnerError, Wasm};

const PROPOSAL_DURATION: u64 = 1000;

#[test]
fn test_update_owner_staking() {
    let env = StakingEnv::new();

    let wasm = Wasm::new(&env.app);

    let fee_collector = env.deploy_fee_collector_contract(&wasm, "margined-collector".to_string());
    let staking = env.deploy_staking_contract(&wasm, "margined-staking".to_string(), fee_collector);

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
