use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::ErrorKind;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use std::{fmt, thread};

use wnf::{AsWnfState, BorrowedWnfState, OwnedWnfState, WnfChangeStamp, WnfDataAccessor};

macro_rules! apply_tests {
    ($($name:ident: $state:ident => $apply:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let state = Arc::new(OwnedWnfState::<u32>::create_temporary().unwrap());
                state.set(&0).unwrap();

                const NUM_THREADS: u32 = 2;
                const NUM_ITERATIONS: u32 = 128;

                let mut handles = Vec::new();

                for _ in 0..NUM_THREADS {
                    let state = Arc::clone(&state);

                    handles.push(thread::spawn(move || {
                        for _ in 0..NUM_ITERATIONS {
                            let $state = state.as_wnf_state();
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
    state.set(&[0, 0]).unwrap();

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

    state.set(&0).unwrap();
    let result = state.try_apply(|v| Ok::<_, TestError>(v + 1)).unwrap();

    assert_eq!(result, 1);
    assert_eq!(state.get().unwrap(), 1);
}

#[test]
fn try_apply_by_value_err() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    state.set(&0).unwrap();
    let result = state.try_apply(|_| Err::<u32, _>(TestError));

    assert!(matches!(result, Err(err) if err.kind() == ErrorKind::Other));
}

#[test]
fn try_apply_boxed_ok() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    state.set(&0).unwrap();
    let result = state.try_apply_boxed(|v| Ok::<_, TestError>(*v + 1)).unwrap();

    assert_eq!(result, 1);
    assert_eq!(state.get().unwrap(), 1);
}

#[test]
fn try_apply_boxed_err() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    state.set(&0).unwrap();
    let result = state.try_apply_boxed(|_| Err::<u32, _>(TestError));

    assert!(matches!(result, Err(err) if err.kind() == ErrorKind::Other));
}

#[test]
fn try_apply_slice_ok() {
    let state = OwnedWnfState::<[u32]>::create_temporary().unwrap();

    state.set(&[0]).unwrap();
    let result = state
        .try_apply_boxed(|vs| Ok::<_, TestError>(vs.iter().map(|v| v + 1).collect::<Vec<_>>()))
        .unwrap();

    assert_eq!(result, [1]);
    assert_eq!(*state.get_boxed().unwrap(), [1]);
}

#[test]
fn try_apply_slice_err() {
    let state = OwnedWnfState::<[u32]>::create_temporary().unwrap();

    state.set(&[0]).unwrap();
    let result = state.try_apply_boxed(|_| Err::<Vec<_>, _>(TestError));

    assert!(matches!(result, Err(err) if err.kind() == ErrorKind::Other));
}

#[test]
fn replace() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    state.set(&0).unwrap();
    let old_value = state.replace(&1).unwrap();

    assert_eq!(state.get().unwrap(), 1);
    assert_eq!(old_value, 0);
}

#[test]
fn replace_boxed() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();

    state.set(&0).unwrap();
    let old_value = state.replace_boxed(&1).unwrap();

    assert_eq!(state.get().unwrap(), 1);
    assert_eq!(*old_value, 0);
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
    assert!(state.exists().unwrap());

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
                .subscribe(move |accessor: WnfDataAccessor<_>| {
                    tx.send(accessor.query().unwrap()).unwrap();
                })
                .unwrap(),
        )
    }

    drop(tx);

    for i in 0..3 {
        state.set(&i).unwrap();

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
        subscription.unsubscribe().unwrap();
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
                .subscribe(move |accessor: WnfDataAccessor<_>| {
                    tx.send(accessor.query_boxed().unwrap()).unwrap();
                })
                .unwrap(),
        )
    }

    drop(tx);

    for i in 0..3 {
        state.set(&i).unwrap();

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
        subscription.unsubscribe().unwrap();
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
                .subscribe(move |accessor: WnfDataAccessor<_>| {
                    tx.send(accessor.query_boxed().unwrap()).unwrap();
                })
                .unwrap(),
        )
    }

    drop(tx);

    for i in 0..3 {
        state.set(&[i, i]).unwrap();

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
        subscription.unsubscribe().unwrap();
    }

    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1)),
        Err(RecvTimeoutError::Disconnected)
    );
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct TestError;

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for TestError {}
