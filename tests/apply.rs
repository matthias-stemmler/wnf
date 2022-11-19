use std::error::Error;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;
use std::thread;
use std::{fmt::Display, io::ErrorKind};

use wnf::{AsState, OwnedState};

macro_rules! apply_tests {
    ($($name:ident: $state:ident => $apply:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let state = Arc::new(OwnedState::<u32>::create_temporary().unwrap());
                state.set(&0).unwrap();

                const NUM_THREADS: u32 = 2;
                const NUM_ITERATIONS: u32 = 128;

                let mut handles = Vec::new();

                for _ in 0..NUM_THREADS {
                    let state = Arc::clone(&state);

                    handles.push(thread::spawn(move || {
                        for _ in 0..NUM_ITERATIONS {
                            let $state = state.as_state();
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
    let state = Arc::new(OwnedState::<[u32]>::create_temporary().unwrap());
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
    let state = OwnedState::<u32>::create_temporary().unwrap();

    state.set(&0).unwrap();
    let result = state.try_apply(|v| Ok::<_, TestError>(v + 1)).unwrap();

    assert_eq!(result, 1);
    assert_eq!(state.get().unwrap(), 1);
}

#[test]
fn try_apply_by_value_err() {
    let state = OwnedState::<u32>::create_temporary().unwrap();

    state.set(&0).unwrap();
    let result = state.try_apply(|_| Err::<u32, _>(TestError));

    assert!(matches!(result, Err(err) if err.kind() == ErrorKind::Other));
}

#[test]
fn try_apply_boxed_ok() {
    let state = OwnedState::<u32>::create_temporary().unwrap();

    state.set(&0).unwrap();
    let result = state.try_apply_boxed(|v| Ok::<_, TestError>(*v + 1)).unwrap();

    assert_eq!(result, 1);
    assert_eq!(state.get().unwrap(), 1);
}

#[test]
fn try_apply_boxed_err() {
    let state = OwnedState::<u32>::create_temporary().unwrap();

    state.set(&0).unwrap();
    let result = state.try_apply_boxed(|_| Err::<u32, _>(TestError));

    assert!(matches!(result, Err(err) if err.kind() == ErrorKind::Other));
}

#[test]
fn try_apply_slice_ok() {
    let state = OwnedState::<[u32]>::create_temporary().unwrap();

    state.set(&[0]).unwrap();
    let result = state
        .try_apply_boxed(|vs| Ok::<_, TestError>(vs.iter().map(|v| v + 1).collect::<Vec<_>>()))
        .unwrap();

    assert_eq!(result, [1]);
    assert_eq!(*state.get_boxed().unwrap(), [1]);
}

#[test]
fn try_apply_slice_err() {
    let state = OwnedState::<[u32]>::create_temporary().unwrap();

    state.set(&[0]).unwrap();
    let result = state.try_apply_boxed(|_| Err::<Vec<_>, _>(TestError));

    assert!(matches!(result, Err(err) if err.kind() == ErrorKind::Other));
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct TestError;

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for TestError {}
