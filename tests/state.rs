use wnf::{BorrowedWnfState, OwnedWnfState, WnfOpaqueData};

#[test]
fn owned_wnf_state_drop_deletes_state() {
    let state = OwnedWnfState::<()>::create_temporary().unwrap();
    assert!(state.exists().unwrap());

    let state_name = state.state_name();
    drop(state);

    let state = BorrowedWnfState::<()>::from_state_name(state_name);
    assert!(!state.exists().unwrap());
}

#[test]
fn owned_wnf_state_leak_does_not_delete_state() {
    let state = {
        let owned_state = OwnedWnfState::<()>::create_temporary().unwrap();
        owned_state.leak()
    };
    assert!(state.exists().unwrap());

    state.to_owned_wnf_state();
}

#[test]
fn owned_wnf_state_cast_does_not_delete_state() {
    let state = OwnedWnfState::<()>::create_temporary().unwrap();
    assert!(state.exists().unwrap());

    let state = state.cast::<WnfOpaqueData>();
    assert!(state.exists().unwrap());
}
