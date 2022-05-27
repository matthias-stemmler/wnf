use wnf::{BorrowedWnfState, OwnedWnfState, WnfDataScope, WnfStateNameDescriptor, WnfStateNameLifetime};

#[test]
fn create_temporary() {
    let state: OwnedWnfState = OwnedWnfState::create_temporary().unwrap();
    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Temporary);
    assert_eq!(state_name_descriptor.data_scope, WnfDataScope::Machine);
    assert!(!state_name_descriptor.is_permanent);
}

#[test]
fn set_and_get_by_value() {
    let state: OwnedWnfState = OwnedWnfState::create_temporary().unwrap();

    let value = 0x12345678;
    state.set(&value).unwrap();
    let read_value: u32 = state.get().unwrap();

    assert_eq!(read_value, value);
}

#[test]
fn set_and_get_boxed() {
    let state: OwnedWnfState = OwnedWnfState::create_temporary().unwrap();

    let value = Box::new(0x12345678);
    state.set(&*value).unwrap();
    let read_value: Box<u32> = state.get_boxed().unwrap();

    assert_eq!(read_value, value);
}

#[test]
fn set_and_get_slice() {
    let state: OwnedWnfState = OwnedWnfState::create_temporary().unwrap();

    let value = vec![0x12345678, 0xABCDEF01, 0x23456789];
    state.set_slice(&value).unwrap();
    let read_value: Box<[u32]> = state.get_slice().unwrap();

    assert_eq!(*read_value, *value);
}

macro_rules! apply_tests {
    ($($name:ident: $apply:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let state: OwnedWnfState = OwnedWnfState::create_temporary().unwrap();
                state.set(&0u32).unwrap();

                let num_threads = 2;
                let num_iterations = 128;

                crossbeam::thread::scope(|s| {
                    for _ in 0..num_threads {
                        s.spawn(|_| {
                            for _ in 0..num_iterations {
                                $apply(state.borrow()).unwrap();
                            }
                        });
                    }
                })
                .unwrap();

                let read_value: u32 = state.get().unwrap();
                assert_eq!(read_value, num_threads * num_iterations);
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
    let state: OwnedWnfState = OwnedWnfState::create_temporary().unwrap();
    state.set_slice(&[0u32, 0u32]).unwrap();

    let num_threads = 2;
    let num_iterations = 128;

    crossbeam::thread::scope(|s| {
        for _ in 0..num_threads {
            s.spawn(|_| {
                for _ in 0..num_iterations {
                    state
                        .apply_slice(|vs: Box<[u32]>| vs.iter().map(|v| v + 1).collect::<Vec<_>>())
                        .unwrap();
                }
            });
        }
    })
    .unwrap();

    let read_value: [u32; 2] = state.get().unwrap();
    let expected_value = num_threads * num_iterations;
    assert_eq!(read_value, [expected_value, expected_value]);
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
