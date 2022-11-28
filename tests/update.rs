use wnf::{ChangeStamp, OwnedState};

#[test]
fn set() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    let value = 0x12345678;

    state.set(&value).unwrap();

    let (read_value, change_stamp) = state.query().unwrap().into_data_change_stamp();
    assert_eq!(read_value, value);
    assert_eq!(change_stamp.value(), 1);
}

#[test]
fn set_slice() {
    let state = OwnedState::<[u32]>::create_temporary().unwrap();
    let slice = [0x12345678, 0xABCDEF01, 0x23456789];

    state.set(&slice).unwrap();

    let (read_slice, change_stamp) = state.query_boxed().unwrap().into_data_change_stamp();
    assert_eq!(*read_slice, slice);
    assert_eq!(change_stamp.value(), 1);
}

#[test]
fn update() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    assert_eq!(state.change_stamp().unwrap(), ChangeStamp::initial());

    let updated = state.update(&0x11111111, ChangeStamp::initial()).unwrap();
    assert!(updated);
    let (read_value, change_stamp) = state.query().unwrap().into_data_change_stamp();
    assert_eq!(read_value, 0x11111111);
    assert_eq!(change_stamp.value(), 1);

    let updated = state.update(&0x22222222, ChangeStamp::from(1)).unwrap();
    assert!(updated);
    let (read_value, change_stamp) = state.query().unwrap().into_data_change_stamp();
    assert_eq!(read_value, 0x22222222);
    assert_eq!(change_stamp.value(), 2);

    let updated = state.update(&0x33333333, ChangeStamp::from(1)).unwrap();
    assert!(!updated);
    let (read_value, change_stamp) = state.query().unwrap().into_data_change_stamp();
    assert_eq!(read_value, 0x22222222);
    assert_eq!(change_stamp.value(), 2);
}
