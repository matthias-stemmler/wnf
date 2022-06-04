use std::sync::mpsc::RecvTimeoutError;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use wnf::{
    BorrowedWnfState, OwnedWnfState, WnfApplyError, WnfChangeStamp, WnfDataScope, WnfStateNameDescriptor,
    WnfStateNameLifetime, WnfTransformError,
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

    let value: u32 = 0x12345678;
    state.set(value).unwrap();

    let read_value: u32 = state.get().unwrap();
    assert_eq!(read_value, value);
}

#[test]
fn get_boxed() {
    let state = OwnedWnfState::create_temporary().unwrap();

    let value: u32 = 0x12345678;
    state.set(value).unwrap();

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
    apply_value_to_value: |state: BorrowedWnfState| state.apply(|v: u32| Some(v + 1)),
    apply_value_to_boxed: |state: BorrowedWnfState| state.apply(|v: u32| Some(Box::new(v + 1))),
    apply_boxed_to_value: |state: BorrowedWnfState| state.apply_boxed(|v: Box<u32>| Some(*v + 1)),
    apply_boxed_to_boxed: |state: BorrowedWnfState| state.apply_boxed(|v: Box<u32>| Some(Box::new(*v + 1))),
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
                    .apply_slice(|vs: Box<[u32]>| Some(vs.iter().map(|v| v + 1).collect::<Vec<_>>()))
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
fn apply_early_termination() {
    let state = OwnedWnfState::create_temporary().unwrap();

    state.set(0u32).unwrap();
    let applied = state.apply::<u32, u32, _>(|_| None).unwrap();

    assert!(!applied);
    assert_eq!(state.get::<u32>().unwrap(), 0);
}

#[test]
fn try_apply_by_value_ok() {
    let state = OwnedWnfState::create_temporary().unwrap();

    state.set(0u32).unwrap();
    let result = state.try_apply::<_, _, TestError, _>(|v: u32| Ok(Some(v + 1)));

    assert_eq!(result, Ok(true));
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
    let result = state.try_apply_boxed::<_, _, TestError, _>(|v: Box<u32>| Ok(Some(*v + 1)));

    assert_eq!(result, Ok(true));
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
    let result = state
        .try_apply_slice::<_, _, TestError, _>(|vs: Box<[u32]>| Ok(Some(vs.iter().map(|v| v + 1).collect::<Vec<_>>())));

    assert_eq!(result, Ok(true));
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
    let owned_state = OwnedWnfState::create_temporary().unwrap();
    let state_name = owned_state.state_name();
    drop(owned_state);

    let borrowed_state: BorrowedWnfState = BorrowedWnfState::from_state_name(state_name);
    assert!(!borrowed_state.exists().unwrap());
}

#[test]
fn owned_wnf_state_delete() {
    let owned_state = OwnedWnfState::create_temporary().unwrap();
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

#[test]
fn subscribe() {
    let owned_state = OwnedWnfState::create_temporary().unwrap();
    let (tx, rx) = mpsc::channel();
    let mut subscriptions = Vec::new();

    const NUM_SUBSCRIPTIONS: usize = 2;

    for _ in 0..NUM_SUBSCRIPTIONS {
        let tx = tx.clone();
        subscriptions.push(
            owned_state
                .subscribe(
                    WnfChangeStamp::initial(),
                    Box::new(move |data: Option<u32>, change_stamp| {
                        tx.send((data, change_stamp)).unwrap();
                    }),
                )
                .unwrap(),
        )
    }

    drop(tx);

    for i in 0..3 {
        owned_state.set(i).unwrap();

        for _ in 0..NUM_SUBSCRIPTIONS {
            let (data, change_stamp) = rx.recv_timeout(Duration::from_secs(1)).unwrap();
            assert_eq!(data, Some(i));
            assert_eq!(change_stamp, WnfChangeStamp::from(i + 1));
        }
    }

    for subscription in subscriptions {
        subscription.unsubscribe().map_err(|(err, _)| err).unwrap();
    }

    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1)),
        Err(RecvTimeoutError::Disconnected)
    );
}

#[test]
fn exists() {
    let owned_state = OwnedWnfState::create_temporary().unwrap();

    let exists = owned_state.exists().unwrap();

    assert!(exists);
}

#[test]
fn not_exists() {
    let borrowed_state = BorrowedWnfState::from_state_name(
        WnfStateNameDescriptor {
            version: 1,
            lifetime: WnfStateNameLifetime::Temporary,
            data_scope: WnfDataScope::Machine,
            is_permanent: false,
            unique_id: 1 << 53 - 1,
        }
        .try_into()
        .unwrap(),
    );

    let exists = borrowed_state.exists().unwrap();

    assert!(!exists);
}

#[test]
fn subscribers_present() {
    let owned_state = OwnedWnfState::create_temporary().unwrap();
    assert!(!owned_state.subscribers_present().unwrap());

    let subscription = owned_state
        .subscribe::<u32, _, _>(WnfChangeStamp::initial(), Box::new(|| {}))
        .unwrap();
    assert!(owned_state.subscribers_present().unwrap());

    subscription.unsubscribe().map_err(|(err, _)| err).unwrap();
    assert!(!owned_state.subscribers_present().unwrap());
}

#[test]
fn is_quiescent() {
    let owned_state = OwnedWnfState::create_temporary().unwrap();
    let (tx, rx) = mpsc::channel();

    let subscription = owned_state
        .subscribe::<(), _, _>(
            WnfChangeStamp::initial(),
            Box::new(move || {
                let _ = rx.recv();
            }),
        )
        .unwrap();

    assert!(owned_state.is_quiescent().unwrap());

    owned_state.set(()).unwrap();
    assert!(!owned_state.is_quiescent().unwrap());

    tx.send(()).unwrap();
    subscription.unsubscribe().map_err(|(err, _)| err).unwrap();

    assert!(owned_state.is_quiescent().unwrap());
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct TestError;
