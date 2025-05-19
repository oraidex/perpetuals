use crate::contract::{execute, instantiate, query};
use crate::testing::new_shutdown_scenario;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{from_binary, Addr, StdError, SubMsg, Uint128};
use margined_common::asset::AssetInfo;
use margined_perp::margined_insurance_fund::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, OwnerResponse, QueryMsg,
};
use margined_utils::cw_multi_test::Executor;
use margined_utils::testing::ShutdownScenario;

const ENGINE: &str = "engine";

#[test]
fn test_instantiation() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        engine: ENGINE.to_string(),
    };
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            engine: Addr::unchecked(ENGINE.to_string()),
        }
    );
}

#[test]
fn test_update_owner() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        engine: ENGINE.to_string(),
    };
    let info = mock_info("addr0000", &[]);

    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Update the owner
    let msg = ExecuteMsg::UpdateOwner {
        owner: "addr0001".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetOwner {}).unwrap();
    let resp: OwnerResponse = from_binary(&res).unwrap();
    let owner = resp.owner;

    assert_eq!(owner, Addr::unchecked("addr0001".to_string()));
}

#[test]
fn test_query_vamm() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        ..
    } = new_shutdown_scenario();

    // add vamm to vammlist in insurance_fund here
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner, msg).unwrap();

    // query if the vamm has been added
    let is_vamm = insurance_fund
        .is_vamm(&router.wrap(), vamm1.addr().to_string())
        .unwrap();

    assert_eq!(is_vamm, true);
}

#[test]
fn test_query_all_vamm() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        vamm2,
        ..
    } = new_shutdown_scenario();

    // check to see that there are no vAMMs
    let res = insurance_fund.all_vamms(&router.wrap(), None).unwrap_err();

    assert_eq!(
        res.to_string(),
        "Generic error: Querier contract error: Generic error: No vAMMs are stored"
    );

    // add a vAMM
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // add another vAMM
    let msg = insurance_fund.add_vamm(vamm2.addr().to_string()).unwrap();
    router.execute(owner, msg).unwrap();

    // check for the added vAMMs
    let res = insurance_fund.all_vamms(&router.wrap(), None).unwrap();
    let list = res.vamm_list;

    assert_eq!(list, vec![vamm1.addr(), vamm2.addr()]);
}

#[test]
fn test_add_vamm() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        ..
    } = new_shutdown_scenario();

    // query the vAMM we want to add
    let is_vamm = insurance_fund
        .is_vamm(&router.wrap(), vamm1.addr().to_string())
        .unwrap();

    assert_eq!(is_vamm, false);

    // add vamm to vammlist in insurance_fund here
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner, msg).unwrap();

    // check for the added vAMM
    let is_vamm = insurance_fund
        .is_vamm(&router.wrap(), vamm1.addr().to_string())
        .unwrap();

    assert_eq!(is_vamm, true);
}

#[test]
fn test_add_vamm_twice() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        ..
    } = new_shutdown_scenario();

    // add vamm to vammlist in insurance_fund here
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // try to add the same vamm here
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    let err = router.execute(owner, msg).unwrap_err();
    assert_eq!(
        StdError::GenericErr {
            msg: "This vAMM is already added".to_string(),
        },
        err.downcast().unwrap()
    );
}

#[test]
fn test_add_second_vamm() {
    // this tests for adding a second vAMM, to ensure the 'push' match arm of save_vamm is used
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        vamm2,
        ..
    } = new_shutdown_scenario();

    // add first vamm to vammlist in insurance_fund here
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // add second vamm to vammlist in insurance_fund here
    let msg = insurance_fund.add_vamm(vamm2.addr().to_string()).unwrap();
    router.execute(owner, msg).unwrap();

    // check for the second added vAMM
    let is_vamm = insurance_fund
        .is_vamm(&router.wrap(), vamm2.addr().to_string())
        .unwrap();

    assert_eq!(is_vamm, true);
}

#[test]
fn test_remove_vamm() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        ..
    } = new_shutdown_scenario();

    // add first vamm to vammlist in insurance_fund here
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // check to see that there is one vAMM
    let is_vamm = insurance_fund
        .is_vamm(&router.wrap(), vamm1.addr().to_string())
        .unwrap();

    assert_eq!(is_vamm, true);

    // remove the first vAMM
    let msg = insurance_fund
        .remove_vamm(vamm1.addr().to_string())
        .unwrap();
    router.execute(owner, msg).unwrap();

    // check that there are zero AMMs
    let is_vamm = insurance_fund
        .is_vamm(&router.wrap(), vamm1.addr().to_string())
        .unwrap();

    assert_eq!(is_vamm, false);
}

#[test]
fn test_remove_no_vamms() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        ..
    } = new_shutdown_scenario();

    // check to see that there is no vAMM
    let is_vamm = insurance_fund
        .is_vamm(&router.wrap(), vamm1.addr().to_string())
        .unwrap();

    assert_eq!(is_vamm, false);

    // remove the first vAMM
    let msg = insurance_fund
        .remove_vamm(vamm1.addr().to_string())
        .unwrap();
    let err = router.execute(owner, msg).unwrap_err();
    assert_eq!(
        StdError::GenericErr {
            msg: "No vAMMs are stored".to_string(),
        },
        err.downcast().unwrap()
    );
}

#[test]
fn test_remove_non_existed_vamm() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        vamm2,
        ..
    } = new_shutdown_scenario();

    // add first vamm to vammlist in insurance_fund here
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // check to see that there is one vAMM
    let is_vamm = insurance_fund
        .is_vamm(&router.wrap(), vamm1.addr().to_string())
        .unwrap();

    assert_eq!(is_vamm, true);

    // remove a vAMM which isn't stored
    let msg = insurance_fund
        .remove_vamm(vamm2.addr().to_string())
        .unwrap();
    let err = router.execute(owner, msg).unwrap_err();
    assert_eq!(
        StdError::GenericErr {
            msg: "This vAMM has not been added".to_string(),
        },
        err.downcast().unwrap()
    );
}

#[test]
fn test_off_vamm_off_again() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        ..
    } = new_shutdown_scenario();

    // add vamm (remember it is default added as 'on')
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    //turn vamm off
    let msg = insurance_fund.shutdown_vamms().unwrap();
    router.execute(owner.clone(), msg).unwrap();

    //turn vamm off again (note the unauthorized error comes from state.open == open)
    let msg = insurance_fund.shutdown_vamms().unwrap();
    let err = router.execute(owner, msg).unwrap_err();
    assert_eq!(
        StdError::GenericErr {
            msg: "unauthorized".to_string(),
        },
        err.downcast().unwrap()
    );
}

#[test]
fn test_vamm_shutdown() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        vamm2,
        vamm3,
        ..
    } = new_shutdown_scenario();

    // add vamm
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // add second vamm
    let msg = insurance_fund.add_vamm(vamm2.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // add third vamm
    let msg = insurance_fund.add_vamm(vamm3.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // query all vamms' status
    let res = insurance_fund
        .all_vamm_status(&router.wrap(), None)
        .unwrap();
    let vamms_status = res.vamm_list_status;

    assert_eq!(
        vamms_status,
        vec![
            (vamm1.addr(), true),
            (vamm2.addr(), true),
            (vamm3.addr(), true)
        ]
    );

    // shutdown all vamms
    let msg = insurance_fund.shutdown_vamms().unwrap();
    router.execute(owner, msg).unwrap();

    // query all vamms' status
    let res = insurance_fund
        .all_vamm_status(&router.wrap(), None)
        .unwrap();
    let vamms_status = res.vamm_list_status;

    assert_eq!(
        vamms_status,
        vec![
            (vamm1.addr(), false),
            (vamm2.addr(), false),
            (vamm3.addr(), false)
        ]
    );
}

#[test]
fn test_vamm_shutdown_from_insurance() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        vamm2,
        vamm3,
        ..
    } = new_shutdown_scenario();

    // add vamm
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // add second vamm
    let msg = insurance_fund.add_vamm(vamm2.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // add third vamm
    let msg = insurance_fund.add_vamm(vamm3.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // query all vamms' status
    let res = insurance_fund
        .all_vamm_status(&router.wrap(), None)
        .unwrap();
    let vamms_status = res.vamm_list_status;

    assert_eq!(
        vamms_status,
        vec![
            (vamm1.addr(), true),
            (vamm2.addr(), true),
            (vamm3.addr(), true)
        ]
    );

    // shutdown all vamms
    let msg = insurance_fund.shutdown_vamms().unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // query all vamms' status
    let res = insurance_fund
        .all_vamm_status(&router.wrap(), None)
        .unwrap();
    let vamms_status = res.vamm_list_status;

    assert_eq!(
        vamms_status,
        vec![
            (vamm1.addr(), false),
            (vamm2.addr(), false),
            (vamm3.addr(), false)
        ]
    );
}

#[test]
fn test_query_vamm_status() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        ..
    } = new_shutdown_scenario();

    // add vamm
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // query vamm status
    let res = insurance_fund
        .vamm_status(&router.wrap(), vamm1.addr().to_string())
        .unwrap();
    let status = res.vamm_status;

    assert_eq!(status, true);

    // shutdown vamm
    let msg = insurance_fund.shutdown_vamms().unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // query vamm status
    let res = insurance_fund
        .vamm_status(&router.wrap(), vamm1.addr().to_string())
        .unwrap();
    let status = res.vamm_status;

    assert_eq!(status, false);
}

#[test]
fn test_all_vamm_status() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        vamm2,
        ..
    } = new_shutdown_scenario();

    // query all vamms' status (there aren't any yet)
    let res = insurance_fund
        .all_vamm_status(&router.wrap(), None)
        .unwrap_err();

    assert_eq!(
        res.to_string(),
        "Generic error: Querier contract error: Generic error: No vAMMs are stored"
    );

    // add vamm
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // add another vamm
    let msg = insurance_fund.add_vamm(vamm2.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // query all vamms' status
    let res = insurance_fund
        .all_vamm_status(&router.wrap(), None)
        .unwrap();
    let vamms_status = res.vamm_list_status;

    assert_eq!(
        vamms_status,
        vec![(vamm1.addr(), true), (vamm2.addr(), true)]
    );

    // switch first vamm off
    let msg = insurance_fund.shutdown_vamms().unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // query all vamms' status
    let res = insurance_fund
        .all_vamm_status(&router.wrap(), None)
        .unwrap();
    let vamms_status = res.vamm_list_status;

    assert_eq!(
        vamms_status,
        vec![(vamm1.addr(), false), (vamm2.addr(), false)]
    );
}

#[test]
fn test_pagination() {
    // note that this test is superfluous because DEFAULT_PAGINATION_LIMIT > MAX_PAGINATION_LIMIT (this tests default pagi limit)

    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        vamm2,
        ..
    } = new_shutdown_scenario();

    // add vamm
    let msg = insurance_fund.add_vamm(vamm1.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    //add second vamm
    let msg = insurance_fund.add_vamm(vamm2.addr().to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    //query only the first vamm (because we gave it limit of 1)
    let res = insurance_fund
        .all_vamm_status(&router.wrap(), Some(1u32))
        .unwrap();
    let vamms_status = res.vamm_list_status;

    assert_eq!(vamms_status, vec![(vamm1.addr(), true)]);
}
#[test]
fn test_pagination_limit() {
    // for the purpose of this test, VAMM_LIMIT is set to 3 (so four exceeds it!)

    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm1,
        vamm2,
        vamm3,
        ..
    } = new_shutdown_scenario();

    let vamms = vec![
        vamm1.addr().to_string(),
        vamm2.addr().to_string(),
        vamm3.addr().to_string(),
    ];

    // add three vamms
    for n in 1..4 {
        let msg = insurance_fund.add_vamm(vamms[n - 1].clone()).unwrap();
        router.execute(owner.clone(), msg).unwrap();
    }

    // query all vamms status
    let res = insurance_fund
        .all_vamm_status(&router.wrap(), None)
        .unwrap();
    let vamms_status = res.vamm_list_status;

    assert_eq!(
        vamms_status,
        vec![
            (vamm1.addr(), true),
            (vamm2.addr(), true),
            (vamm3.addr(), true),
        ]
    );

    //query only the first two vamms
    let res = insurance_fund
        .all_vamm_status(&router.wrap(), Some(2u32))
        .unwrap();
    let vamms_status = res.vamm_list_status;

    assert_eq!(
        vamms_status,
        vec![(vamm1.addr(), true), (vamm2.addr(), true),]
    );
}

#[test]
fn test_not_owner() {
    //instantiate contract here
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        engine: "orai1xwx3xs6gx9gkgf4rj7wu2elqa92cjqhutlnqx68eppgs09qm8c2qs72jh5".to_string(),
    };
    let info = mock_info("owner", &[]);

    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update relayer
    let msg = ExecuteMsg::UpdateRelayer {
        relayer: "addr0001".to_string(),
    };

    let info = mock_info("owner", &[]);
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // try to update the config
    let msg = ExecuteMsg::UpdateOwner {
        owner: "addr0001".to_string(),
    };

    let info = mock_info("not_the_owner", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

    assert_eq!(res.to_string(), "Generic error: Caller is not admin");

    // try to add a vAMM
    let addr1 = "addr0001".to_string();

    let info = mock_info("not_the_owner", &[]);
    let msg = ExecuteMsg::AddVamm { vamm: addr1 };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

    assert_eq!(res.to_string(), "Generic error: unauthorized");

    //try to remove a vAMM
    let addr1 = "addr0001".to_string();

    let info = mock_info("not_the_owner", &[]);
    let msg = ExecuteMsg::RemoveVamm { vamm: addr1 };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

    assert_eq!(res.to_string(), "Generic error: unauthorized");

    //try to shutdown all vamms
    let info = mock_info("not_the_owner", &[]);
    let msg = ExecuteMsg::ShutdownVamms {};

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

    assert_eq!(res.to_string(), "Generic error: unauthorized");
}

#[test]
fn test_incompatible_decimals() {
    let ShutdownScenario {
        mut router,
        owner,
        insurance_fund,
        vamm5,
        ..
    } = new_shutdown_scenario();

    let msg = insurance_fund.add_vamm(vamm5.addr().to_string()).unwrap();
    let err = router.execute(owner, msg).unwrap_err();
    assert_eq!(
        StdError::GenericErr {
            msg: "vAMM decimals incompatible with margin engine".to_string(),
        },
        err.downcast().unwrap()
    );
}

#[test]
fn tet_withdraw_fund_to_operator() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        engine: ENGINE.to_string(),
    };
    let info = mock_info("owner", &[]);

    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token = AssetInfo::Token {
        contract_addr: Addr::unchecked("usdc"),
    };
    let msg = ExecuteMsg::WithdrawFund {
        token: token.clone(),
        amount: Uint128::one(),
    };

    // withdraw fund failed, unauthorized
    let info = mock_info("addr0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(err, StdError::generic_err("unauthorized"));

    // withdraw fund to operator successful
    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            ("action", "insurance_withdraw_to_operator"),
            ("amount", &Uint128::one().to_string())
        ]
    );
    assert_eq!(
        res.messages,
        vec![SubMsg::new(
            token
                .into_msg("owner".to_string(), Uint128::one(), None)
                .unwrap()
        )]
    )
}
