use wnf::{OpaqueData, OwnedState};

#[test]
fn get() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    let value = 0x12345678;
    state.set(&value).unwrap();

    let read_value = state.get().unwrap();
    
    assert_eq!(read_value, value);
}

#[test]
fn get_boxed_slice() {
    let state = OwnedState::<[u32]>::create_temporary().unwrap();
    let slice = [0x12345678, 0xABCDEF01, 0x23456789];
    state.set(slice.as_slice()).unwrap();

    let read_slice = state.get_boxed().unwrap();

    assert_eq!(*read_slice, slice);
}

#[test]
fn query() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    let value = 0x12345678;
    state.set(&value).unwrap();

    let (read_value, change_stamp) = state.query().unwrap().into_data_change_stamp();
    
    assert_eq!(read_value, value);
    assert_eq!(change_stamp.value(), 1);
}

#[test]
fn query_boxed_slice() {
    let state = OwnedState::<[u32]>::create_temporary().unwrap();
    let slice = [0x12345678, 0xABCDEF01, 0x23456789];
    state.set(slice.as_slice()).unwrap();

    let (read_slice, change_stamp) = state.query_boxed().unwrap().into_data_change_stamp();
    
    assert_eq!(*read_slice, slice);
    assert_eq!(change_stamp.value(), 1);
}

#[test]
fn change_stamp() {
    let state = OwnedState::<u32>::create_temporary().unwrap();

    assert_eq!(state.change_stamp().unwrap().value(), 0);

    state.set(&12345678).unwrap();

    assert_eq!(state.change_stamp().unwrap().value(), 1);
}

#[test]
fn query_opaque_data() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    state.set(&12345678).unwrap();
    let state: OwnedState<OpaqueData> = state.cast();

    assert_eq!(state.query().unwrap().change_stamp().value(), 1);
}
