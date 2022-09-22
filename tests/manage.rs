use wnf::{
    BorrowedWnfState, OwnedWnfState, WnfCreatableStateLifetime, WnfDataScope, WnfOpaqueData, WnfStateCreation,
    WnfStateNameDescriptor, WnfStateNameLifetime,
};

#[test]
fn owned_state_create_temporary() {
    let state = OwnedWnfState::<WnfOpaqueData>::create_temporary().unwrap();
    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Temporary);
    assert_eq!(state_name_descriptor.data_scope, WnfDataScope::Machine);
    assert!(!state_name_descriptor.is_permanent);
}

#[test]
fn borrowed_state_create_temporary() {
    let state = BorrowedWnfState::<WnfOpaqueData>::create_temporary().unwrap();
    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Temporary);
    assert_eq!(state_name_descriptor.data_scope, WnfDataScope::Machine);
    assert!(!state_name_descriptor.is_permanent);

    state.delete().unwrap()
}

macro_rules! create_state_with_scope_tests {
    ($($name:ident: $scope:expr,)*) => {
        $(
            #[test]
            fn $name() {
                create_state_with_scope_test($scope);
            }
        )*
    };
}

create_state_with_scope_tests![
    create_state_with_system_scope: WnfDataScope::System,
    create_state_with_session_scope: WnfDataScope::Session,
    create_state_with_user_scope: WnfDataScope::User,
    create_state_with_machine_scope: WnfDataScope::Machine,
    // WnfDataScope::Process requires SeCreatePermanentPrivilege, see system.rs
];

fn create_state_with_scope_test(scope: WnfDataScope) {
    let state = WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Temporary)
        .scope(scope)
        .create_owned::<WnfOpaqueData>()
        .unwrap();

    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Temporary);
    assert_eq!(state_name_descriptor.data_scope, scope);
    assert!(!state_name_descriptor.is_permanent);
}

#[test]
fn create_state_with_maximum_state_size() {
    let state = WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Temporary)
        .scope(WnfDataScope::Machine)
        .maximum_state_size(1)
        .create_owned::<[u8]>()
        .unwrap();

    assert!(state.set(&[1]).is_ok());
    assert!(state.set(&[1, 2]).is_err());
}

#[test]
fn create_state_with_maximum_state_size_at_limit() {
    assert!(WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Temporary)
        .scope(WnfDataScope::Machine)
        .maximum_state_size(0x1000)
        .create_owned::<WnfOpaqueData>()
        .is_ok());

    assert!(WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Temporary)
        .scope(WnfDataScope::Machine)
        .maximum_state_size(0x1001)
        .create_owned::<WnfOpaqueData>()
        .is_err());
}

#[test]
fn create_state_with_type_id() {
    let state = WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Temporary)
        .scope(WnfDataScope::Machine)
        .type_id("b75fa6ba-77fd-4790-b825-1715ffefbac8")
        .create_owned::<()>()
        .unwrap();

    assert!(state.set(&()).is_ok());

    let borrowed_state_with_wrong_type_id =
        BorrowedWnfState::from_state_name_and_type_id(state.state_name(), "ee26d6d2-53f4-4230-9c9e-88556e82c3d3");

    assert!(borrowed_state_with_wrong_type_id.set(&()).is_err());

    drop(borrowed_state_with_wrong_type_id);
}

#[test]
fn owned_state_delete() {
    let state = OwnedWnfState::<WnfOpaqueData>::create_temporary().unwrap();
    let state_name = state.state_name();

    assert!(state.exists().unwrap());

    state.delete().unwrap();

    let borrowed_state_after_deletion = BorrowedWnfState::<WnfOpaqueData>::from_state_name(state_name);
    assert!(!borrowed_state_after_deletion.exists().unwrap());
}

#[test]
fn borrowed_state_delete() {
    let state = BorrowedWnfState::<WnfOpaqueData>::create_temporary().unwrap();

    assert!(state.exists().unwrap());

    state.delete().unwrap();

    assert!(!state.exists().unwrap());
}
