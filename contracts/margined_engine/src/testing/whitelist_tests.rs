use cosmwasm_std::{Addr, StdError, Uint128};
use margined_perp::margined_engine::Side;
use margined_utils::{
    cw_multi_test::Executor,
    testing::{to_decimals, SimpleScenario},
};

use crate::testing::new_simple_scenario;

#[test]
fn test_add_remove_whitelist() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        ..
    } = new_simple_scenario();

    // add alice to whitelist
    let msg = engine.add_whitelist(alice.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let whitelist = engine.get_whitelist(&router.wrap()).unwrap();
    println!("whitelist: {:?}", whitelist);
    assert_eq!(whitelist, vec![alice.to_string()]);

    // add addr that is already in
    let msg = engine.add_whitelist(alice.to_string()).unwrap();
    let err = router.execute(owner.clone(), msg).unwrap_err();

    assert_eq!(
        StdError::GenericErr {
            msg: "Given address already registered as a hook".to_string(),
        },
        err.downcast().unwrap()
    );

    // remove alice from whitelist
    let msg = engine.remove_whitelist(alice.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let whitelist = engine.get_whitelist(&router.wrap()).unwrap();
    let empty: Vec<String> = Vec::new();

    assert_eq!(whitelist, empty);

    // test remove non-existed addr
    let msg = engine.remove_whitelist(alice.to_string()).unwrap();
    let err = router.execute(owner.clone(), msg).unwrap_err();

    assert_eq!(
        StdError::GenericErr {
            msg: "Given address not registered as a hook".to_string(),
        },
        err.downcast().unwrap()
    );
}

#[test]
fn test_add_remove_then_add_whitelist() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        ..
    } = new_simple_scenario();

    // add alice to whitelist
    let msg = engine.add_whitelist(alice.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let whitelist = engine.get_whitelist(&router.wrap()).unwrap();

    assert_eq!(whitelist, vec![alice.to_string()]);

    // remove alice from whitelist
    let msg = engine.remove_whitelist(alice.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let whitelist = engine.get_whitelist(&router.wrap()).unwrap();
    let empty: Vec<String> = Vec::new();

    assert_eq!(whitelist, empty);

    // add alice to whitelist
    let msg = engine.add_whitelist(alice.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let whitelist = engine.get_whitelist(&router.wrap()).unwrap();

    assert_eq!(whitelist, vec![alice.to_string()]);
}

#[test]
fn test_not_admin() {
    let SimpleScenario {
        mut router,
        alice,
        engine,
        ..
    } = new_simple_scenario();

    // test add as non-admin
    let not_owner = Addr::unchecked("not_owner");

    let msg = engine.add_whitelist(alice.to_string()).unwrap();
    let err = router.execute(not_owner.clone(), msg).unwrap_err();

    assert_eq!(
        StdError::GenericErr {
            msg: "Caller is not admin".to_string(),
        },
        err.downcast().unwrap()
    );

    // test remove as non-admin
    let msg = engine.remove_whitelist(alice.to_string()).unwrap();
    let err = router.execute(not_owner.clone(), msg).unwrap_err();

    assert_eq!(
        StdError::GenericErr {
            msg: "Caller is not admin".to_string(),
        },
        err.downcast().unwrap()
    );
}

#[test]
fn test_query_all_whitelist_and_is_whitelist() {
    let SimpleScenario {
        mut router,
        alice,
        bob,
        owner,
        engine,
        ..
    } = new_simple_scenario();

    // add alice to whitelist
    let msg = engine.add_whitelist(alice.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let whitelist = engine.get_whitelist(&router.wrap()).unwrap();

    assert_eq!(whitelist, vec![alice.to_string()]);

    // add bob to whitelist, alice already in
    let msg = engine.add_whitelist(bob.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    let whitelist = engine.get_whitelist(&router.wrap()).unwrap();

    assert_eq!(whitelist, vec![alice.to_string(), bob.to_string()]);

    // test if alice is in whitelist
    let bool = engine
        .is_whitelist(&router.wrap(), alice.to_string())
        .unwrap();

    assert!(bool);
    // test if bob is in whitelist
    let bool = engine
        .is_whitelist(&router.wrap(), bob.to_string())
        .unwrap();

    assert!(bool);
}

#[test]
fn test_whitelist_works_open_short_over_limit() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    // add alice to whitelist
    let msg = engine.add_whitelist(alice.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // set base asset holding cap
    let msg = vamm.set_base_asset_holding_cap(to_decimals(10u64)).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // open a short over the cap
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Sell,
            to_decimals(100u64),
            to_decimals(1u64),
            Some(to_decimals(6)),
            Some(to_decimals(10)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let res = router.execute(alice.clone(), msg);

    assert!(res.is_ok())
}

#[test]
fn test_whitelist_works_open_long_over_limit() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    // add alice to whitelist
    let msg = engine.add_whitelist(alice.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // set base asset holding cap
    let msg = vamm.set_base_asset_holding_cap(to_decimals(10u64)).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // open a long over the cap
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(100u64),
            to_decimals(1u64),
            Some(to_decimals(14)),
            Some(to_decimals(8)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let res = router.execute(alice.clone(), msg);

    assert!(res.is_ok())
}

#[test]
fn test_whitelist_works_open_short_into_reverse_long() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    // add alice to whitelist
    let msg = engine.add_whitelist(alice.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // set base asset holding cap
    let msg = vamm.set_base_asset_holding_cap(to_decimals(10u64)).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // open a short
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Sell,
            to_decimals(5u64),
            to_decimals(1u64),
            Some(to_decimals(6)),
            Some(to_decimals(10)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    // open a reverse long over the cap
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(100u64),
            to_decimals(1u64),
            Some(to_decimals(13)),
            Some(to_decimals(9)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let res = router.execute(alice.clone(), msg);

    assert!(res.is_ok())
}

#[test]
fn test_whitelist_works_open_long_into_reverse_short() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    // add alice to whitelist
    let msg = engine.add_whitelist(alice.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // set base asset holding cap
    let msg = vamm.set_base_asset_holding_cap(to_decimals(10u64)).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // open a long
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(5u64),
            to_decimals(1u64),
            Some(to_decimals(14)),
            Some(Uint128::zero()),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    // open a reverse short over the cap
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Sell,
            to_decimals(100u64),
            to_decimals(1u64),
            Some(to_decimals(5)),
            Some(to_decimals(10)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let res = router.execute(alice.clone(), msg);

    assert!(res.is_ok())
}

#[test]
fn test_whitelist_works_blocks_short_into_reverse_long() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    // set base asset holding cap
    let msg = vamm
        .set_base_asset_holding_cap(Uint128::from(10_000_000_000u128))
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // open a short
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Sell,
            to_decimals(9u64),
            to_decimals(1u64),
            Some(to_decimals(6)),
            Some(to_decimals(10)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    // open a reverse long over the cap
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(21u64),
            to_decimals(10u64),
            Some(to_decimals(16)),
            Some(to_decimals(7)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();

    assert_eq!(
        StdError::GenericErr {
            msg: "base asset holding exceeds cap".to_string(),
        },
        err.downcast().unwrap()
    );
}

#[test]
fn test_whitelist_blocks_open_long_into_reverse_short() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    // set base asset holding cap
    let msg = vamm.set_base_asset_holding_cap(to_decimals(10u64)).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // open a long
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(5u64),
            to_decimals(1u64),
            Some(to_decimals(13)),
            Some(to_decimals(9)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    // open a reverse short over the cap
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Sell,
            to_decimals(100u64),
            to_decimals(1u64),
            Some(to_decimals(6)),
            Some(to_decimals(10)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();

    assert_eq!(
        StdError::GenericErr {
            msg: "base asset holding exceeds cap".to_string(),
        },
        err.downcast().unwrap()
    );
}

#[test]
fn test_whitelist_no_limit_notional_cap() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    // add alice to whitelist
    let msg = engine.add_whitelist(alice.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // set base asset holding cap
    let msg = vamm
        .set_open_interest_notional_cap(to_decimals(10u64))
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // open a long over the cap
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(100u64),
            to_decimals(1u64),
            Some(to_decimals(16)),
            Some(to_decimals(10)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let res = router.execute(alice.clone(), msg);

    assert!(res.is_ok())
}

#[test]
fn test_whitelist_wont_stop_trading_if_reduce_pos() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        vamm,
        ..
    } = new_simple_scenario();

    // add alice to whitelist
    let msg = engine.add_whitelist(alice.to_string()).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // set base asset holding cap
    let msg = vamm
        .set_open_interest_notional_cap(to_decimals(10u64))
        .unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // open a long over the cap
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Buy,
            to_decimals(100u64),
            to_decimals(1u64),
            Some(to_decimals(15)),
            Some(to_decimals(10)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    router.execute(alice.clone(), msg).unwrap();

    // open a short to reduce posn, still over cap
    let msg = engine
        .open_position(
            vamm.addr().to_string(),
            Side::Sell,
            to_decimals(10u64),
            to_decimals(1u64),
            Some(to_decimals(7)),
            Some(to_decimals(12)),
            to_decimals(0u64),
            vec![],
        )
        .unwrap();
    let res = router.execute(alice.clone(), msg);

    assert!(res.is_ok())
}

#[test]
fn test_whitelist_relayer() {
    let SimpleScenario {
        mut router,
        alice,
        owner,
        engine,
        ..
    } = new_simple_scenario();

    // add alice to whitelist fail, unathorized
    let msg = engine.set_relayer(vec![alice.clone()]).unwrap();
    let err = router.execute(alice.clone(), msg).unwrap_err();

    assert_eq!(
        StdError::GenericErr {
            msg: "Unauthorized".to_string(),
        },
        err.downcast().unwrap()
    );

    // add alice to whitelist success
    let msg = engine.set_relayer(vec![alice.clone()]).unwrap();
    router.execute(owner.clone(), msg).unwrap();

    // remove alice from whitelist success
    let msg = engine.remove_relayer(vec![alice.clone()]).unwrap();
    router.execute(owner.clone(), msg).unwrap();
}
