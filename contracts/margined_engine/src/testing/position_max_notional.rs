use cosmwasm_std::{StdError, Uint128};
use cw20::Cw20ExecuteMsg;
use margined_common::integer::Integer;
use margined_perp::margined_engine::{
    PnlCalcOption, PositionFilter, Side, TickResponse, TicksResponse,
};
use margined_utils::{
    cw_multi_test::Executor,
    testing::{to_decimals, SimpleScenario},
};

use crate::{contract::TRANSFER_FAILURE_REPLY_ID, testing::new_simple_scenario};

#[test]
fn test_open_position_long_exceeds_max_notional_size() {
    let SimpleScenario {
        mut router,
        owner,
        alice,
        usdc,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    let msg = engine
        .update_trading_config(None, Some(to_decimals(500)), None)
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(60u64),
            to_decimals(10u64),
            Some(to_decimals(18)),
            Some(to_decimals(9)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        "Generic error: Notional size exceeds max notional size".to_string()
    );

    // set max notional size to 1000
    let msg = engine
        .update_trading_config(None, Some(to_decimals(1000)), None)
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(60u64),
            to_decimals(10u64),
            Some(to_decimals(18)),
            Some(to_decimals(9)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    // expect to be 60
    let margin = engine
        .get_balance_with_funding_payment(&router.wrap(), 1)
        .unwrap();
    assert_eq!(margin, to_decimals(60));

    // personal position should be 37.5
    let position = engine
        .position(&router.wrap(), vamm.addr().to_string(), 1)
        .unwrap();
    println!("position.notional: {:?}", position.notional);
    assert_eq!(position.size, Integer::new_positive(37_500_000_000u128)); //37_500_000_000 // 600_000_000_000
    assert_eq!(position.margin, to_decimals(60u64));

    // clearing house token balance should be 60
    let engine_balance = usdc.balance(&router.wrap(), engine.addr().clone()).unwrap();
    assert_eq!(engine_balance, to_decimals(60));
}

#[test]
fn test_open_position_shorts_exceeds_max_notional_size() {
    let SimpleScenario {
        mut router,
        owner,
        alice,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    let msg = engine
        .update_trading_config(None, Some(to_decimals(150)), None)
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Sell,
            to_decimals(40u64),
            to_decimals(5u64),
            Some(to_decimals(7)),
            Some(to_decimals(13)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        "Generic error: Notional size exceeds max notional size".to_string()
    );

    // set max notional size to 500
    let msg = engine
        .update_trading_config(None, Some(to_decimals(500)), None)
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Sell,
            to_decimals(40u64),
            to_decimals(5u64),
            Some(to_decimals(7)),
            Some(to_decimals(13)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Sell,
            to_decimals(40u64),
            to_decimals(5u64),
            Some(to_decimals(4)),
            Some(to_decimals(8)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    // personal balance with funding payment
    let margin_1 = engine
        .get_balance_with_funding_payment(&router.wrap(), 1)
        .unwrap();
    let margin_2 = engine
        .get_balance_with_funding_payment(&router.wrap(), 2)
        .unwrap();
    assert_eq!(margin_1 + margin_2, to_decimals(80));

    // retrieve the vamm state
    let position_1 = engine
        .position(&router.wrap(), vamm.addr().to_string(), 1)
        .unwrap();
    let position_2 = engine
        .position(&router.wrap(), vamm.addr().to_string(), 2)
        .unwrap();
    assert_eq!(
        position_1.size + position_2.size,
        Integer::new_negative(66_666_666_667u128)
    );
    assert_eq!(position_1.margin + position_2.margin, to_decimals(80));
}
