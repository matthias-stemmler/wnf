use std::sync::mpsc;

use wnf::{
    BorrowedWnfState, OwnedWnfState, WnfDataAccessor, WnfDataScope, WnfSeenChangeStamp, WnfStateName,
    WnfStateNameDescriptor, WnfStateNameLifetime,
};

#[test]
fn exists() {
    let state = OwnedWnfState::<()>::create_temporary().unwrap();

    let exists = state.exists().unwrap();

    assert!(exists);
}

#[test]
fn not_exists() {
    let state = BorrowedWnfState::<()>::from_state_name(
        WnfStateName::try_from(WnfStateNameDescriptor {
            version: 1,
            lifetime: WnfStateNameLifetime::Temporary,
            data_scope: WnfDataScope::Machine,
            is_permanent: false,
            unique_id: 0,
            owner_tag: 1, // this must be `0` for non-well-known state names, so such a state name cannot exist
        })
        .unwrap(),
    );

    let exists = state.exists().unwrap();

    assert!(!exists);
}

#[test]
fn subscribers_present() {
    let state = OwnedWnfState::<()>::create_temporary().unwrap();
    assert!(!state.subscribers_present().unwrap());

    let subscription = state
        .subscribe(|_: WnfDataAccessor<_>| {}, WnfSeenChangeStamp::None)
        .unwrap();
    assert!(state.subscribers_present().unwrap());

    subscription.unsubscribe().unwrap();
    assert!(!state.subscribers_present().unwrap());
}

#[test]
fn is_quiescent() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();
    let (tx, rx) = mpsc::channel();

    let subscription = state
        .subscribe(
            move |_: WnfDataAccessor<_>| {
                let _ = rx.recv();
            },
            WnfSeenChangeStamp::None,
        )
        .unwrap();

    assert!(state.is_quiescent().unwrap());

    state.set(&42).unwrap();
    assert!(!state.is_quiescent().unwrap());

    tx.send(()).unwrap();
    subscription.unsubscribe().unwrap();

    assert!(state.is_quiescent().unwrap());
}
