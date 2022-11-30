use wnf::{
    BorrowedState, DataAccessor, DataScope, OwnedState, SeenChangeStamp, StateName, StateNameDescriptor,
    StateNameLifetime,
};

#[test]
fn exists() {
    let state = OwnedState::<()>::create_temporary().unwrap();

    let exists = state.exists().unwrap();

    assert!(exists);
}

#[test]
fn not_exists() {
    let state = BorrowedState::<()>::from_state_name(
        StateName::try_from(StateNameDescriptor {
            version: 1,
            lifetime: StateNameLifetime::Temporary,
            data_scope: DataScope::Machine,
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
    let state = OwnedState::<()>::create_temporary().unwrap();
    assert!(!state.subscribers_present().unwrap());

    let subscription = state.subscribe(|_: DataAccessor<_>| {}, SeenChangeStamp::None).unwrap();
    assert!(state.subscribers_present().unwrap());

    subscription.unsubscribe().unwrap();
    assert!(!state.subscribers_present().unwrap());
}

#[test]
fn is_quiescent() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    let (tx, rx) = crossbeam_channel::unbounded();

    let subscription = state
        .subscribe(
            move |_: DataAccessor<_>| {
                let _ = rx.recv();
            },
            SeenChangeStamp::None,
        )
        .unwrap();

    assert!(state.is_quiescent().unwrap());

    state.set(&42).unwrap();
    assert!(!state.is_quiescent().unwrap());

    tx.send(()).unwrap();
    subscription.unsubscribe().unwrap();
    assert!(state.is_quiescent().unwrap());
}
