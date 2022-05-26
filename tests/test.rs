use wnf::{BorrowedWnfState, OwnedWnfState, WnfDataScope, WnfStateNameDescriptor, WnfStateNameLifetime};

#[test]
fn create_temporary() {
    let state: OwnedWnfState = OwnedWnfState::create_temporary().unwrap();
    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Temporary);
    assert_eq!(state_name_descriptor.data_scope, WnfDataScope::Machine);
    assert!(!state_name_descriptor.is_permanent);
}

#[test]
fn set_and_get() {
    let state: OwnedWnfState = OwnedWnfState::create_temporary().unwrap();

    state.set(&0x12345678).unwrap();
    let value: u32 = state.get().unwrap();

    assert_eq!(value, 0x12345678);
}

#[test]
fn owned_wnf_state_drop_deletes_state() {
    let owned_state: OwnedWnfState = OwnedWnfState::create_temporary().unwrap();
    let state_name = owned_state.state_name();
    drop(owned_state);

    let borrowed_state: BorrowedWnfState = BorrowedWnfState::from_state_name(state_name);
    assert!(!borrowed_state.exists().unwrap());
}

#[test]
fn owned_wnf_state_delete() {
    let owned_state: OwnedWnfState = OwnedWnfState::create_temporary().unwrap();
    let state_name = owned_state.state_name();
    owned_state.delete().unwrap();

    let borrowed_state: BorrowedWnfState = BorrowedWnfState::from_state_name(state_name);
    assert!(!borrowed_state.exists().unwrap());
}

#[test]
fn borrowed_wnf_state_delete() {
    let borrowed_state: BorrowedWnfState = OwnedWnfState::create_temporary().unwrap().leak();
    let state_name = borrowed_state.state_name();
    borrowed_state.delete().unwrap();

    let borrowed_state: BorrowedWnfState = BorrowedWnfState::from_state_name(state_name);
    assert!(!borrowed_state.exists().unwrap());
}
