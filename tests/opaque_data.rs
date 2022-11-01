use std::time::Duration;
use wnf::{AsState, DataAccessor, OpaqueData, OwnedState, SeenChangeStamp};

#[test]
fn query() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    state.set(&42).unwrap();

    let state: OwnedState<OpaqueData> = state.cast();

    let change_stamp = state.query().unwrap().change_stamp();
    assert_eq!(change_stamp, 1.into());
}

#[test]
fn subscribe() {
    let state = OwnedState::<OpaqueData>::create_temporary().unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();

    let _subscription = state.subscribe(
        move |accessor: DataAccessor<_>| {
            tx.send(accessor.change_stamp()).unwrap();
        },
        SeenChangeStamp::None,
    );

    state.as_state().cast::<u32>().set(&42).unwrap();
    let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(change_stamp, 1.into());

    state.as_state().cast::<u16>().set(&43).unwrap();
    let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(change_stamp, 2.into());
}
