use cosmwasm_std::{StdError, Uint128};
use cw20::Cw20ExecuteMsg;
use margined_perp::margined_engine::Side;
use margined_utils::{
    cw_multi_test::Executor,
    testing::{to_decimals, SimpleScenario},
};

use crate::{contract::INCREASE_POSITION_REPLY_ID, testing::new_simple_scenario};

#[test]
fn test_force_error_open_position_exceeds_price_diff_limit() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        usdc,
        vamm,
        ..
    } = new_simple_scenario();

    // reduce the allowance
    router
        .execute_contract(
            alice.clone(),
            usdc.addr().clone(),
            &Cw20ExecuteMsg::DecreaseAllowance {
                spender: engine.addr().to_string(),
                amount: to_decimals(1900),
                expires: None,
            },
            &[],
        )
        .unwrap();

    let msg = vamm
        .set_fluctuation_limit_ratio(Uint128::from(300_000_000u128))
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // alice pays 20 margin * 5x long quote when 9.0909091 base
    // AMM after: 1100 : 90.9090909, price: 12.1000000012
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(20u64),
            to_decimals(5u64),
            Some(to_decimals(15)),
            Some(to_decimals(8)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    // curent price diff 20%
    // set price diff limit to 10%
    let msg = vamm
        .set_price_diff_limit_ratio(Uint128::from(100_000_000u128))
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // try to open position again
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(2u64),
            to_decimals(5u64),
            Some(to_decimals(15)),
            Some(to_decimals(8)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();
    assert_eq!(
        StdError::GenericErr {
            msg: "Over price diff limit oracle & vamm price".to_string(),
        },
        err.downcast().unwrap()
    );

    // set price diff limit to 30% & working
    let msg = vamm
        .set_price_diff_limit_ratio(Uint128::from(300_000_000u128))
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();
}

#[test]
fn test_force_error_close_position_exceeds_price_diff_limit() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        usdc,
        vamm,
        ..
    } = new_simple_scenario();

    // reduce the allowance
    router
        .execute_contract(
            alice.clone(),
            usdc.addr().clone(),
            &Cw20ExecuteMsg::DecreaseAllowance {
                spender: engine.addr().to_string(),
                amount: to_decimals(1900),
                expires: None,
            },
            &[],
        )
        .unwrap();

    let msg = vamm
        .set_fluctuation_limit_ratio(Uint128::from(300_000_000u128))
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // alice pays 20 margin * 5x long quote when 9.0909091 base
    // AMM after: 1100 : 90.9090909, price: 12.1000000012
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(20u64),
            to_decimals(5u64),
            Some(to_decimals(15)),
            Some(to_decimals(8)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    // curent price diff 20%
    // set price diff limit to 10%
    let msg = vamm
        .set_price_diff_limit_ratio(Uint128::from(100_000_000u128))
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // close position error
    let msg = engine
        .close_position(vamm.addr().to_string(), 1, to_decimals(118u64))
        .unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();

    assert_eq!(
        StdError::GenericErr {
            msg: "Over price diff limit oracle & vamm price".to_string(),
        },
        err.downcast().unwrap()
    );

    // set price diff limit to 30% & working
    let msg = vamm
        .set_price_diff_limit_ratio(Uint128::from(300_000_000u128))
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();
}
