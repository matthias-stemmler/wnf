use wnf::OwnedState;

#[test]
fn replace() {
    let state = OwnedState::<u32>::create_temporary().unwrap();

    state.set(&0).unwrap();
    let old_value = state.replace(&1).unwrap();

    assert_eq!(state.get().unwrap(), 1);
    assert_eq!(old_value, 0);
}

#[test]
fn replace_boxed() {
    let state = OwnedState::<u32>::create_temporary().unwrap();

    state.set(&0).unwrap();
    let old_value = state.replace_boxed(&1).unwrap();

    assert_eq!(state.get().unwrap(), 1);
    assert_eq!(*old_value, 0);
}
