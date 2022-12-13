use wnf::{BorrowedState, OwnedState};

#[test]
fn owned_state_drop_deletes_state() {
    let state = OwnedState::<()>::create_temporary().unwrap();
    assert!(state.exists().unwrap());

    let state_name = state.state_name();
    drop(state);

    let state = BorrowedState::<()>::from_state_name(state_name);
    assert!(!state.exists().unwrap());
}

#[test]
fn owned_state_leak_does_not_delete_state() {
    let state = {
        let owned_state = OwnedState::<()>::create_temporary().unwrap();
        owned_state.leak()
    };
    assert!(state.exists().unwrap());

    state.to_owned_state();
}

#[test]
fn owned_state_cast_does_not_delete_state() {
    let state = OwnedState::<()>::create_temporary().unwrap();
    assert!(state.exists().unwrap());

    let state = state.cast::<()>();
    assert!(state.exists().unwrap());
}
