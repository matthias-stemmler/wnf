use wnf::{CreatableStateLifetime, DataScope, StateCreation, StateLifetime, StateNameDescriptor};

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
    let state = StateCreation::new()
        .lifetime(CreatableStateLifetime::Persistent)
        .scope(DataScope::Machine)
        .create_owned::<()>()
        .unwrap();

    let state_name_descriptor: StateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, StateLifetime::Persistent);
    assert_eq!(state_name_descriptor.data_scope, DataScope::Machine);
    assert!(!state_name_descriptor.is_permanent);
    assert_eq!(state_name_descriptor.owner_tag, 0);
}

fn create_state_with_permanent_lifetime_and_non_persistent_data() {
    let state = StateCreation::new()
        .lifetime(CreatableStateLifetime::Permanent { persist_data: false })
        .scope(DataScope::Machine)
        .create_owned::<()>()
        .unwrap();

    let state_name_descriptor: StateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, StateLifetime::Permanent);
    assert_eq!(state_name_descriptor.data_scope, DataScope::Machine);
    assert!(!state_name_descriptor.is_permanent);
    assert_eq!(state_name_descriptor.owner_tag, 0);
}

fn create_state_with_permanent_lifetime_and_persistent_data() {
    let state = StateCreation::new()
        .lifetime(CreatableStateLifetime::Permanent { persist_data: true })
        .scope(DataScope::Machine)
        .create_owned::<()>()
        .unwrap();

    let state_name_descriptor: StateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, StateLifetime::Permanent);
    assert_eq!(state_name_descriptor.data_scope, DataScope::Machine);
    assert!(state_name_descriptor.is_permanent);
    assert_eq!(state_name_descriptor.owner_tag, 0);
}

fn create_state_with_process_scope() {
    let state = StateCreation::new()
        .lifetime(CreatableStateLifetime::Persistent)
        .scope(DataScope::Process)
        .create_owned::<()>()
        .unwrap();

    let state_name_descriptor: StateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, StateLifetime::Persistent);
    assert_eq!(state_name_descriptor.data_scope, DataScope::Process);
    assert!(!state_name_descriptor.is_permanent);
    assert_eq!(state_name_descriptor.owner_tag, 0);
}
