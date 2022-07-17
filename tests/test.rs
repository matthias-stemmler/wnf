use std::sync::mpsc::RecvTimeoutError;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use wnf::{
    BorrowAsWnfState, BorrowedWnfState, OwnedWnfState, WnfApplyError, WnfChangeStamp, WnfDataScope,
    WnfStateNameDescriptor, WnfStateNameLifetime, WnfTransformError,
};

#[test]
fn create_temporary() {
    let state = OwnedWnfState::<()>::create_temporary().unwrap();
    let state_name_descriptor: WnfStateNameDescriptor = state.state_name().try_into().unwrap();

    assert_eq!(state_name_descriptor.version, 1);
    assert_eq!(state_name_descriptor.lifetime, WnfStateNameLifetime::Temporary);
    assert_eq!(state_name_descriptor.data_scope, WnfDataScope::Machine);
    assert!(!state_name_descriptor.is_permanent);
}

#[test]
fn set_by_value() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    let value = 0x12345678;
    state.set(value).unwrap();

    let read_value = state.get().unwrap();
    assert_eq!(read_value, value);
}

#[test]
fn set_by_ref() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    let value = 0x12345678;
    state.set(&value).unwrap();

    let read_value = state.get().unwrap();
    assert_eq!(read_value, value);
}

#[test]
fn set_boxed() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    let value = 0x12345678;
    state.set(Box::new(value)).unwrap();

    let read_value = state.get().unwrap();
    assert_eq!(read_value, value);
}

#[test]
fn set_slice_by_ref() {
    let state = OwnedWnfState::<[u32]>::create_temporary().unwrap();

    let values = [0x12345678, 0xABCDEF01, 0x23456789];
    state.set(values.as_slice()).unwrap();

    let read_slice = state.get_boxed().unwrap();
    assert_eq!(*read_slice, values);
}

#[test]
fn set_slice_vec() {
    let state = OwnedWnfState::<[u32]>::create_temporary().unwrap();

    let values = [0x12345678, 0xABCDEF01, 0x23456789];
    state.set(values.to_vec()).unwrap();

    let read_slice = state.get_boxed().unwrap();
    assert_eq!(*read_slice, values);
}

#[test]
fn get_by_value() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    let value = 0x12345678;
    state.set(value).unwrap();

    let read_value = state.get().unwrap();
    assert_eq!(read_value, value);
}

#[test]
fn get_boxed() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    let value = 0x12345678;
    state.set(value).unwrap();

    let read_value = state.get_boxed().unwrap();
    assert_eq!(*read_value, value);
}

#[test]
fn get_slice() {
    let state = OwnedWnfState::<[u32]>::create_temporary().unwrap();

    let values = [0x12345678, 0xABCDEF01, 0x23456789];
    state.set(values.as_slice()).unwrap();

    let read_values = state.get_boxed().unwrap();
    assert_eq!(*read_values, values);
}

macro_rules! apply_tests {
    ($($name:ident: $state:ident => $apply:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let state = Arc::new(OwnedWnfState::<u32>::create_temporary().unwrap());
                state.set(0).unwrap();

                const NUM_THREADS: u32 = 2;
                const NUM_ITERATIONS: u32 = 128;

                let mut handles = Vec::new();

                for _ in 0..NUM_THREADS {
                    let state = Arc::clone(&state);

                    handles.push(thread::spawn(move || {
                        for _ in 0..NUM_ITERATIONS {
                            let $state = state.borrow_as_wnf_state();
                            $apply.unwrap();
                        }
                    }));
                }

                for handle in handles {
                    handle.join().unwrap();
                }

                let read_value = state.get().unwrap();
                assert_eq!(read_value, NUM_THREADS * NUM_ITERATIONS);
            }
        )*
    };
}

apply_tests! {
    apply_value_to_value: state => state.apply(|v| v + 1),
    apply_value_to_boxed: state => state.apply(|v| Box::new(v + 1)),
    apply_boxed_to_value: state => state.apply_boxed(|v| *v + 1),
    apply_boxed_to_boxed: state => state.apply_boxed(|v| Box::new(*v + 1)),
}

#[test]
fn apply_slice_to_vec() {
    let state = Arc::new(OwnedWnfState::<[u32]>::create_temporary().unwrap());
    state.set([0, 0]).unwrap();

    const NUM_THREADS: u32 = 2;
    const NUM_ITERATIONS: u32 = 128;

    let mut handles = Vec::new();

    for _ in 0..NUM_THREADS {
        let state = Arc::clone(&state);

        handles.push(thread::spawn(move || {
            for _ in 0..NUM_ITERATIONS {
                state
                    .apply_boxed(|vs| vs.iter().map(|v| v + 1).collect::<Vec<_>>())
                    .unwrap();
            }
        }))
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let read_value = state.get_boxed().unwrap();
    let expected_value = NUM_THREADS * NUM_ITERATIONS;
    assert_eq!(*read_value, [expected_value, expected_value]);
}

#[test]
fn try_apply_by_value_ok() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    state.set(0).unwrap();
    let result = state.try_apply(|v| Ok::<_, TestError>(v + 1)).unwrap();

    assert_eq!(result, 1);
    assert_eq!(state.get().unwrap(), 1);
}

#[test]
fn try_apply_by_value_err() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    state.set(0).unwrap();
    let result = state.try_apply(|_| Err::<u32, _>(TestError));

    assert_eq!(result, Err(WnfApplyError::Transform(WnfTransformError(TestError))));
}

#[test]
fn try_apply_boxed_ok() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    state.set(0).unwrap();
    let result = state.try_apply_boxed(|v| Ok::<_, TestError>(*v + 1)).unwrap();

    assert_eq!(result, 1);
    assert_eq!(state.get().unwrap(), 1);
}

#[test]
fn try_apply_boxed_err() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    state.set(0).unwrap();
    let result = state.try_apply_boxed(|_| Err::<u32, _>(TestError));

    assert_eq!(result, Err(WnfApplyError::Transform(WnfTransformError(TestError))));
}

#[test]
fn try_apply_slice_ok() {
    let state = OwnedWnfState::<[u32]>::create_temporary().unwrap();

    state.set([0]).unwrap();
    let result = state
        .try_apply_boxed(|vs| Ok::<_, TestError>(vs.iter().map(|v| v + 1).collect::<Vec<_>>()))
        .unwrap();

    assert_eq!(result, [1]);
    assert_eq!(*state.get_boxed().unwrap(), [1]);
}

#[test]
fn try_apply_slice_err() {
    let state = OwnedWnfState::<[u32]>::create_temporary().unwrap();

    state.set([0]).unwrap();
    let result = state.try_apply_boxed(|_| Err::<Vec<_>, _>(TestError));

    assert_eq!(result, Err(WnfApplyError::Transform(WnfTransformError(TestError))));
}

#[test]
fn replace() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    state.set(0).unwrap();
    let old_value = state.replace(1).unwrap();

    assert_eq!(state.get().unwrap(), 1);
    assert_eq!(old_value, 0);
}

#[test]
fn replace_boxed() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    state.set(0).unwrap();
    let old_value = state.replace_boxed(1).unwrap();

    assert_eq!(state.get().unwrap(), 1);
    assert_eq!(*old_value, 0);
}

#[test]
fn owned_wnf_state_drop_deletes_state() {
    let state = OwnedWnfState::<()>::create_temporary().unwrap();
    let state_name = state.state_name();
    drop(state);

    let state = BorrowedWnfState::<()>::from_state_name(state_name);
    assert!(!state.exists().unwrap());
}

#[test]
fn owned_wnf_state_delete() {
    let state = OwnedWnfState::<()>::create_temporary().unwrap();
    let state_name = state.state_name();
    state.delete().unwrap();

    let state = BorrowedWnfState::<()>::from_state_name(state_name);
    assert!(!state.exists().unwrap());
}

#[test]
fn borrowed_wnf_state_delete() {
    let state = OwnedWnfState::<()>::create_temporary().unwrap().leak();
    let state_name = state.state_name();
    state.delete().unwrap();

    let state = BorrowedWnfState::<()>::from_state_name(state_name);
    assert!(!state.exists().unwrap());
}

#[test]
fn subscribe() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();
    let (tx, rx) = mpsc::channel();
    let mut subscriptions = Vec::new();

    const NUM_SUBSCRIPTIONS: usize = 2;

    for _ in 0..NUM_SUBSCRIPTIONS {
        let tx = tx.clone();
        subscriptions.push(
            state
                .subscribe(WnfChangeStamp::initial(), move |accessor| {
                    tx.send(accessor.query().unwrap()).unwrap();
                })
                .unwrap(),
        )
    }

    drop(tx);

    for i in 0..3 {
        state.set(i).unwrap();

        for _ in 0..NUM_SUBSCRIPTIONS {
            let (data, change_stamp) = rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .into_data_change_stamp();

            assert_eq!(data, i);
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
fn subscribe_boxed() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();
    let (tx, rx) = mpsc::channel();
    let mut subscriptions = Vec::new();

    const NUM_SUBSCRIPTIONS: usize = 2;

    for _ in 0..NUM_SUBSCRIPTIONS {
        let tx = tx.clone();
        subscriptions.push(
            state
                .subscribe(WnfChangeStamp::initial(), move |accessor| {
                    tx.send(accessor.query_boxed().unwrap()).unwrap();
                })
                .unwrap(),
        )
    }

    drop(tx);

    for i in 0..3 {
        state.set(i).unwrap();

        for _ in 0..NUM_SUBSCRIPTIONS {
            let (data, change_stamp) = rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .into_data_change_stamp();

            assert_eq!(*data, i);
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
fn subscribe_slice() {
    let state = OwnedWnfState::<[u32]>::create_temporary().unwrap();
    let (tx, rx) = mpsc::channel();
    let mut subscriptions = Vec::new();

    const NUM_SUBSCRIPTIONS: usize = 2;

    for _ in 0..NUM_SUBSCRIPTIONS {
        let tx = tx.clone();
        subscriptions.push(
            state
                .subscribe(WnfChangeStamp::initial(), move |accessor| {
                    tx.send(accessor.query_boxed().unwrap()).unwrap();
                })
                .unwrap(),
        )
    }

    drop(tx);

    for i in 0..3 {
        state.set([i, i]).unwrap();

        for _ in 0..NUM_SUBSCRIPTIONS {
            let (data, change_stamp) = rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .into_data_change_stamp();

            assert_eq!(*data, [i, i]);
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
    let state = OwnedWnfState::<()>::create_temporary().unwrap();

    let exists = state.exists().unwrap();

    assert!(exists);
}

#[test]
fn not_exists() {
    let state = BorrowedWnfState::<()>::from_state_name(
        WnfStateNameDescriptor {
            version: 1,
            lifetime: WnfStateNameLifetime::Temporary,
            data_scope: WnfDataScope::Machine,
            is_permanent: false,
            unique_id: (1 << 53) - 1,
        }
        .try_into()
        .unwrap(),
    );

    let exists = state.exists().unwrap();

    assert!(!exists);
}

#[test]
fn subscribers_present() {
    let state = OwnedWnfState::<()>::create_temporary().unwrap();
    assert!(!state.subscribers_present().unwrap());

    let subscription = state.subscribe(WnfChangeStamp::initial(), |_| {}).unwrap();
    assert!(state.subscribers_present().unwrap());

    subscription.unsubscribe().map_err(|(err, _)| err).unwrap();
    assert!(!state.subscribers_present().unwrap());
}

#[test]
fn is_quiescent() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();
    let (tx, rx) = mpsc::channel();

    let subscription = state
        .subscribe(WnfChangeStamp::initial(), move |_| {
            let _ = rx.recv();
        })
        .unwrap();

    assert!(state.is_quiescent().unwrap());

    state.set(42).unwrap();
    assert!(!state.is_quiescent().unwrap());

    tx.send(()).unwrap();
    subscription.unsubscribe().map_err(|(err, _)| err).unwrap();

    assert!(state.is_quiescent().unwrap());
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct TestError;
