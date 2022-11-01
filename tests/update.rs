use wnf::{ChangeStamp, OwnedState};

#[test]
fn set() {
    let state = OwnedState::<u32>::create_temporary().unwrap();

    let value = 0x12345678;
    state.set(&value).unwrap();

    let read_value = state.get().unwrap();
    assert_eq!(read_value, value);
}

#[test]
fn set_slice() {
    let state = OwnedState::<[u32]>::create_temporary().unwrap();

    let values = [0x12345678, 0xABCDEF01, 0x23456789];
    state.set(&values).unwrap();

    let read_slice = state.get_boxed().unwrap();
    assert_eq!(*read_slice, values);
}

#[test]
fn set_with_expected_change_stamp() {
    let state = OwnedState::<u32>::create_temporary().unwrap();

    let updated = state.update(&0x11111111, ChangeStamp::initial()).unwrap();
    assert!(updated);
    assert_eq!(state.get().unwrap(), 0x11111111);

    let updated = state.update(&0x22222222, ChangeStamp::from(1)).unwrap();
    assert!(updated);
    assert_eq!(state.get().unwrap(), 0x22222222);

    let updated = state.update(&0x33333333, ChangeStamp::from(1)).unwrap();
    assert!(!updated);
    assert_eq!(state.get().unwrap(), 0x22222222);

    let updated = state.update(&0x44444444, ChangeStamp::from(3)).unwrap();
    assert!(!updated);
    assert_eq!(state.get().unwrap(), 0x22222222);
}
