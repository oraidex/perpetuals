use margined_perp::margined_engine::{PauseType, Side};
use margined_utils::{
    cw_multi_test::Executor,
    testing::{to_decimals, SimpleScenario},
};

use crate::testing::new_simple_scenario;

#[test]
fn test_paused_all_by_admin() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    let msg = engine.set_pause(PauseType::All).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(1u64),
            to_decimals(1u64),
            Some(to_decimals(11)),
            Some(to_decimals(8)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        "Generic error: Margin engine is paused".to_string()
    );

    let msg = engine
        .deposit_margin(vamm.addr().to_string(), 1, to_decimals(1u64), vec![])
        .unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        "Generic error: Margin engine is paused".to_string()
    );

    let msg = engine
        .withdraw_margin(vamm.addr().to_string(), 1, to_decimals(1u64))
        .unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        "Generic error: Margin engine is paused".to_string()
    );

    let msg = engine
        .close_position(vamm.addr().to_string(), 1, to_decimals(0u64))
        .unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        "margined_perp::margined_engine::Position not found".to_string()
    );
}

#[test]
fn test_paused_open_by_admin() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(1u64),
            to_decimals(1u64),
            Some(to_decimals(11)),
            Some(to_decimals(8)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    let msg = engine.set_pause(PauseType::Open).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(1u64),
            to_decimals(1u64),
            Some(to_decimals(11)),
            Some(to_decimals(8)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        "Generic error: Margin engine is paused".to_string()
    );

    let msg = engine
        .deposit_margin(vamm.addr().to_string(), 1, to_decimals(1u64), vec![])
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    let msg = engine
        .withdraw_margin(vamm.addr().to_string(), 1, to_decimals(1u64))
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    let msg = engine
        .close_position(vamm.addr().to_string(), 1, to_decimals(0u64))
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();
}

#[test]
fn test_paused_close_by_admin() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    let msg = engine.set_pause(PauseType::Close).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(1u64),
            to_decimals(1u64),
            Some(to_decimals(11)),
            Some(to_decimals(8)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    let msg = engine
        .deposit_margin(vamm.addr().to_string(), 1, to_decimals(1u64), vec![])
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    let msg = engine
        .withdraw_margin(vamm.addr().to_string(), 1, to_decimals(1u64))
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    let msg = engine
        .close_position(vamm.addr().to_string(), 1, to_decimals(0u64))
        .unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        "Generic error: Margin engine is paused".to_string()
    );
}

#[test]
fn test_cant_be_paused_by_non_admin() {
    let SimpleScenario {
        mut router,
        alice,
        engine,
        ..
    } = new_simple_scenario();

    let msg = engine.set_pause(PauseType::All).unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        "Generic error: unauthorized".to_string()
    );
}

#[test]
fn test_pause_then_unpause_by_admin() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    let msg = engine.set_pause(PauseType::All).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let msg = engine.set_pause(PauseType::None).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(1u64),
            to_decimals(1u64),
            Some(to_decimals(11)),
            Some(to_decimals(8)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    let msg = engine
        .deposit_margin(vamm.addr().to_string(), 1, to_decimals(1u64), vec![])
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    let msg = engine
        .withdraw_margin(vamm.addr().to_string(), 1, to_decimals(1u64))
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    let msg = engine
        .close_position(vamm.addr().to_string(), 1, to_decimals(0u64))
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();
}

#[test]
fn test_cant_unpause_when_already_unpaused_and_vice_versa() {
    let SimpleScenario {
        mut router,
        owner,
        engine,
        ..
    } = new_simple_scenario();

    let msg = engine.set_pause(PauseType::None).unwrap();
    let err = router.execute(owner.clone(), msg).unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        "Generic error: unauthorized".to_string()
    );

    let msg = engine.set_pause(PauseType::All).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let msg = engine.set_pause(PauseType::All).unwrap();
    let err = router.execute(owner.clone(), msg).unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        "Generic error: unauthorized".to_string()
    );
}
