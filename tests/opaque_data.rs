use std::sync::mpsc::channel;
use std::time::Duration;
use wnf::{AsWnfState, OwnedWnfState, WnfDataAccessor, WnfOpaqueData, WnfSeenChangeStamp};

#[test]
fn query() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();
    state.set(&42).unwrap();

    let state: OwnedWnfState<WnfOpaqueData> = state.cast();

    let change_stamp = state.query().unwrap().change_stamp();
    assert_eq!(change_stamp, 1.into());
}

#[test]
fn subscribe() {
    let state = OwnedWnfState::<WnfOpaqueData>::create_temporary().unwrap();

    let (tx, rx) = channel();

    let _subscription = state.subscribe(
        move |accessor: WnfDataAccessor<_>| {
            tx.send(accessor.change_stamp()).unwrap();
        },
        WnfSeenChangeStamp::None,
    );

    state.as_wnf_state().cast::<u32>().set(&42).unwrap();
    let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(change_stamp, 1.into());

    state.as_wnf_state().cast::<u16>().set(&43).unwrap();
    let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(change_stamp, 2.into());
}
