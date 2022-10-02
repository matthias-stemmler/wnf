use wnf::{
    BorrowedWnfState, BoxedSecurityDescriptor, OwnedWnfState, WnfCreatableStateLifetime, WnfDataScope, WnfOpaqueData,
    WnfStateCreation, WnfStateNameDescriptor, WnfStateNameLifetime,
};

#[test]
fn owned_state_create_temporary() {
    let state = OwnedWnfState::<WnfOpaqueData>::create_temporary().unwrap();
    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Temporary);
    assert_eq!(state_name_descriptor.data_scope, WnfDataScope::Machine);
    assert!(!state_name_descriptor.is_permanent);
    assert_eq!(state_name_descriptor.owner_tag, 0);
}

#[test]
fn borrowed_state_create_temporary() {
    let state = BorrowedWnfState::<WnfOpaqueData>::create_temporary().unwrap();
    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Temporary);
    assert_eq!(state_name_descriptor.data_scope, WnfDataScope::Machine);
    assert!(!state_name_descriptor.is_permanent);
    assert_eq!(state_name_descriptor.owner_tag, 0);

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
    // WnfDataScope::Process is not compatible with WnfStateNameLifetime::Temporary and hence requires
    // SeCreatePermanentPrivilege, see system.rs
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
    assert_eq!(state_name_descriptor.owner_tag, 0);
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
        .create_owned()
        .unwrap();

    assert!(state.set(&()).is_ok());

    let borrowed_state_with_wrong_type_id =
        BorrowedWnfState::from_state_name_and_type_id(state.state_name(), "ee26d6d2-53f4-4230-9c9e-88556e82c3d3");

    assert!(borrowed_state_with_wrong_type_id.set(&()).is_err());

    drop(borrowed_state_with_wrong_type_id);
}

#[test]
fn create_state_with_everyone_generic_all_security_descriptor() {
    let state = WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Temporary)
        .scope(WnfDataScope::Machine)
        .security_descriptor(BoxedSecurityDescriptor::create_everyone_generic_all().unwrap())
        .create_owned()
        .unwrap();

    assert!(state.get().is_ok());
    assert!(state.set(&()).is_ok());
}

#[test]
fn create_state_with_security_descriptor_from_string() {
    let state_creation = WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Temporary)
        .scope(WnfDataScope::Machine);

    let sd_all: BoxedSecurityDescriptor = "D:(A;;GA;;;WD)".parse().unwrap();
    let sd_readonly: BoxedSecurityDescriptor = "D:(A;;GR;;;WD)".parse().unwrap();
    let sd_none: BoxedSecurityDescriptor = "D:(A;;;;;WD)".parse().unwrap();

    let state = state_creation.security_descriptor(sd_all).create_owned().unwrap();

    assert!(state.get().is_ok());
    assert!(state.set(&()).is_ok());

    let state = state_creation.security_descriptor(sd_readonly).create_owned().unwrap();

    assert!(state.get().is_ok());
    assert!(state.set(&()).is_err());

    let state = state_creation.security_descriptor(sd_none).create_owned().unwrap();

    assert!(state.get().is_err());
    assert!(state.set(&()).is_err());
}

#[cfg(feature = "windows-permissions")]
#[test]
fn create_state_with_security_descriptor_from_windows_permissions() {
    use windows_permissions::{LocalBox, SecurityDescriptor};

    let state_creation = WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Temporary)
        .scope(WnfDataScope::Machine);

    let sd_all: LocalBox<SecurityDescriptor> = "D:(A;;GA;;;WD)".parse().unwrap();
    let sd_readonly: LocalBox<SecurityDescriptor> = "D:(A;;GR;;;WD)".parse().unwrap();
    let sd_none: LocalBox<SecurityDescriptor> = "D:(A;;;;;WD)".parse().unwrap();

    let state = state_creation.security_descriptor(sd_all).create_owned().unwrap();

    assert!(state.get().is_ok());
    assert!(state.set(&()).is_ok());

    let state = state_creation.security_descriptor(sd_readonly).create_owned().unwrap();

    assert!(state.get().is_ok());
    assert!(state.set(&()).is_err());

    let state = state_creation.security_descriptor(sd_none).create_owned().unwrap();

    assert!(state.get().is_err());
    assert!(state.set(&()).is_err());
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
