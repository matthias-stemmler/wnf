use std::sync::Arc;
use std::thread;

use wnf::{
    BorrowedWnfState, OwnedWnfState, WnfApplyError, WnfDataScope, WnfStateNameDescriptor, WnfStateNameLifetime,
    WnfTransformError,
};

#[test]
fn create_temporary() {
    let state = OwnedWnfState::create_temporary().unwrap();
    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Temporary);
    assert_eq!(state_name_descriptor.data_scope, WnfDataScope::Machine);
    assert!(!state_name_descriptor.is_permanent);
}

#[test]
fn set_by_value() {
    let state = OwnedWnfState::create_temporary().unwrap();

    let value = 0x12345678;
    state.set(value).unwrap();

    let read_value: u32 = state.get().unwrap();
    assert_eq!(read_value, value);
}

#[test]
fn set_by_ref() {
    let state = OwnedWnfState::create_temporary().unwrap();

    let value = 0x12345678;
    state.set::<u32, _>(&value).unwrap();

    let read_value: u32 = state.get().unwrap();
    assert_eq!(read_value, value);
}

#[test]
fn set_boxed() {
    let state = OwnedWnfState::create_temporary().unwrap();

    let value = 0x12345678;
    state.set::<u32, _>(Box::new(value)).unwrap();

    let read_value: u32 = state.get().unwrap();
    assert_eq!(read_value, value);
}

#[test]
fn set_slice_by_ref() {
    let state = OwnedWnfState::create_temporary().unwrap();

    let values = [0x12345678, 0xABCDEF01, 0x23456789];
    state.set_slice(values.as_slice()).unwrap();

    let read_slice: Box<[u32]> = state.get_slice().unwrap();
    assert_eq!(*read_slice, values);
}

#[test]
fn set_slice_vec() {
    let state = OwnedWnfState::create_temporary().unwrap();

    let values = [0x12345678, 0xABCDEF01, 0x23456789];
    state.set_slice(values.to_vec()).unwrap();

    let read_slice: Box<[u32]> = state.get_slice().unwrap();
    assert_eq!(*read_slice, values);
}

#[test]
fn get_by_value() {
    let state = OwnedWnfState::create_temporary().unwrap();

    let value = 0x12345678;
    state.set(0x12345678).unwrap();

    let read_value: u32 = state.get().unwrap();
    assert_eq!(read_value, value);
}

#[test]
fn get_boxed() {
    let state = OwnedWnfState::create_temporary().unwrap();

    let value = 0x12345678;
    state.set(0x12345678).unwrap();

    let read_value: Box<u32> = state.get_boxed().unwrap();
    assert_eq!(*read_value, value);
}

#[test]
fn get_slice() {
    let state = OwnedWnfState::create_temporary().unwrap();

    let values = [0x12345678, 0xABCDEF01, 0x23456789];
    state.set_slice(values.as_slice()).unwrap();

    let read_values: Box<[u32]> = state.get_slice().unwrap();
    assert_eq!(*read_values, values);
}

macro_rules! apply_tests {
    ($($name:ident: $apply:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let state = Arc::new(OwnedWnfState::create_temporary().unwrap());
                state.set(0u32).unwrap();

                const NUM_THREADS: u32 = 2;
                const NUM_ITERATIONS: u32 = 128;

                let mut handles = Vec::new();

                for _ in 0..NUM_THREADS {
                    let state = Arc::clone(&state);

                    handles.push(thread::spawn(move || {
                        for _ in 0..NUM_ITERATIONS {
                            $apply(state.borrow()).unwrap();
                        }
                    }));
                }

                for handle in handles {
                    handle.join().unwrap();
                }

                let read_value: u32 = state.get().unwrap();
                assert_eq!(read_value, NUM_THREADS * NUM_ITERATIONS);
            }
        )*
    };
}

apply_tests! {
    apply_value_to_value: |state: BorrowedWnfState| state.apply(|v: u32| v + 1),
    apply_value_to_boxed: |state: BorrowedWnfState| state.apply(|v: u32| Box::new(v + 1)),
    apply_boxed_to_value: |state: BorrowedWnfState| state.apply_boxed(|v: Box<u32>| *v + 1),
    apply_boxed_to_boxed: |state: BorrowedWnfState| state.apply_boxed(|v: Box<u32>| Box::new(*v + 1)),
}

#[test]
fn apply_slice_to_vec() {
    let state = Arc::new(OwnedWnfState::create_temporary().unwrap());
    state.set_slice([0u32, 0u32]).unwrap();

    const NUM_THREADS: u32 = 2;
    const NUM_ITERATIONS: u32 = 128;

    let mut handles = Vec::new();

    for _ in 0..NUM_THREADS {
        let state = Arc::clone(&state);

        handles.push(thread::spawn(move || {
            for _ in 0..NUM_ITERATIONS {
                state
                    .apply_slice(|vs: Box<[u32]>| vs.iter().map(|v| v + 1).collect::<Vec<_>>())
                    .unwrap();
            }
        }))
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let read_value: [u32; 2] = state.get().unwrap();
    let expected_value = NUM_THREADS * NUM_ITERATIONS;
    assert_eq!(read_value, [expected_value, expected_value]);
}

#[test]
fn try_apply_by_value_ok() {
    let state = OwnedWnfState::create_temporary().unwrap();

    state.set(0u32).unwrap();
    let result = state.try_apply::<_, _, TestError, _>(|v: u32| Ok(v + 1));

    assert!(result.is_ok());
    assert_eq!(state.get::<u32>().unwrap(), 1);
}

#[test]
fn try_apply_by_value_err() {
    let state = OwnedWnfState::create_temporary().unwrap();

    state.set(0u32).unwrap();
    let result = state.try_apply::<u32, u32, _, _>(|_| Err(TestError));

    assert_eq!(result, Err(WnfApplyError::Transform(WnfTransformError(TestError))));
}

#[test]
fn try_apply_boxed_ok() {
    let state = OwnedWnfState::create_temporary().unwrap();

    state.set(0u32).unwrap();
    let result = state.try_apply_boxed::<_, _, TestError, _>(|v: Box<u32>| Ok(*v + 1));

    assert!(result.is_ok());
    assert_eq!(state.get::<u32>().unwrap(), 1);
}

#[test]
fn try_apply_boxed_err() {
    let state = OwnedWnfState::create_temporary().unwrap();

    state.set(0u32).unwrap();
    let result = state.try_apply_boxed::<u32, u32, _, _>(|_| Err(TestError));

    assert_eq!(result, Err(WnfApplyError::Transform(WnfTransformError(TestError))));
}

#[test]
fn try_apply_slice_ok() {
    let state = OwnedWnfState::create_temporary().unwrap();

    state.set_slice([0u32]).unwrap();
    let result =
        state.try_apply_slice::<_, _, TestError, _>(|vs: Box<[u32]>| Ok(vs.iter().map(|v| v + 1).collect::<Vec<_>>()));

    assert!(result.is_ok());
    assert_eq!(*state.get_slice::<u32>().unwrap(), [1]);
}

#[test]
fn try_apply_slice_err() {
    let state = OwnedWnfState::create_temporary().unwrap();

    state.set_slice([0u32]).unwrap();
    let result = state.try_apply_slice::<u32, Vec<u32>, _, _>(|_| Err(TestError));

    assert_eq!(result, Err(WnfApplyError::Transform(WnfTransformError(TestError))));
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

#[derive(Debug, Eq, Hash, PartialEq)]
struct TestError;
