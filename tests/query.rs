use wnf::{OpaqueData, OwnedState};

#[test]
fn get_by_value() {
    let state = OwnedState::<u32>::create_temporary().unwrap();

    let value = 0x12345678;
    state.set(&value).unwrap();

    let read_value = state.get().unwrap();
    assert_eq!(read_value, value);
}

#[test]
fn get_boxed() {
    let state = OwnedState::<u32>::create_temporary().unwrap();

    let value = 0x12345678;
    state.set(&value).unwrap();

    let read_value = state.get_boxed().unwrap();
    assert_eq!(*read_value, value);
}

#[test]
fn get_slice() {
    let state = OwnedState::<[u32]>::create_temporary().unwrap();

    let values = [0x12345678, 0xABCDEF01, 0x23456789];
    state.set(values.as_slice()).unwrap();

    let read_values = state.get_boxed().unwrap();
    assert_eq!(*read_values, values);
}

#[test]
fn query_opaque_data() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    state.set(&42).unwrap();

    let state: OwnedState<OpaqueData> = state.cast();

    let change_stamp = state.query().unwrap().change_stamp();
    assert_eq!(change_stamp, 1.into());
}
