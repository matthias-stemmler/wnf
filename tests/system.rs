use wnf::{
    WnfCreatableStateLifetime, WnfDataScope, WnfOpaqueData, WnfStateCreation, WnfStateNameDescriptor,
    WnfStateNameLifetime,
};

devutils::system_tests![
    can_create_permanent_shared_objects_returns_true_when_run_as_system,
    create_state_with_persistent_lifetime,
    create_state_with_permanent_lifetime_and_non_persistent_data,
    create_state_with_permanent_lifetime_and_persistent_data,
    create_state_with_process_scope,
];

fn can_create_permanent_shared_objects_returns_true_when_run_as_system() {
    assert!(wnf::can_create_permanent_shared_objects().unwrap());
}

fn create_state_with_persistent_lifetime() {
    let state = WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Persistent)
        .scope(WnfDataScope::Machine)
        .create_owned::<WnfOpaqueData>()
        .unwrap();

    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Persistent);
    assert_eq!(state_name_descriptor.data_scope, WnfDataScope::Machine);
    assert!(!state_name_descriptor.is_permanent);
    assert_eq!(state_name_descriptor.owner_tag, 0);
}

fn create_state_with_permanent_lifetime_and_non_persistent_data() {
    let state = WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Permanent { persist_data: false })
        .scope(WnfDataScope::Machine)
        .create_owned::<WnfOpaqueData>()
        .unwrap();

    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Permanent);
    assert_eq!(state_name_descriptor.data_scope, WnfDataScope::Machine);
    assert!(!state_name_descriptor.is_permanent);
    assert_eq!(state_name_descriptor.owner_tag, 0);
}

fn create_state_with_permanent_lifetime_and_persistent_data() {
    let state = WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Permanent { persist_data: true })
        .scope(WnfDataScope::Machine)
        .create_owned::<WnfOpaqueData>()
        .unwrap();

    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Permanent);
    assert_eq!(state_name_descriptor.data_scope, WnfDataScope::Machine);
    assert!(state_name_descriptor.is_permanent);
    assert_eq!(state_name_descriptor.owner_tag, 0);
}

fn create_state_with_process_scope() {
    let state = WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Persistent)
        .scope(WnfDataScope::Process)
        .create_owned::<WnfOpaqueData>()
        .unwrap();

    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Persistent);
    assert_eq!(state_name_descriptor.data_scope, WnfDataScope::Process);
    assert!(!state_name_descriptor.is_permanent);
    assert_eq!(state_name_descriptor.owner_tag, 0);
}
