use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::io::ErrorKind;
use std::sync::Arc;
use std::thread;

use wnf::OwnedState;

#[test]
fn apply() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    state.set(&0).unwrap();

    let result = state.apply(|value| value + 1).unwrap();

    assert_eq!(result, 1);
    assert_eq!(state.get().unwrap(), 1);
}

#[test]
fn try_apply_ok() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    state.set(&0).unwrap();

    let result = state.try_apply(|value| Ok::<_, TestError>(value + 1)).unwrap();

    assert_eq!(result, 1);
    assert_eq!(state.get().unwrap(), 1);
}

#[test]
fn try_apply_err() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    state.set(&0).unwrap();

    let result = state.try_apply(|_| Err::<u32, _>(TestError));

    assert!(matches!(result, Err(err) if err.kind() == ErrorKind::Other));
}

#[test]
fn apply_concurrent() {
    let state = Arc::new(OwnedState::<u32>::create_temporary().unwrap());
    state.set(&0).unwrap();

    const NUM_THREADS: usize = 2;
    const NUM_ITERATIONS: usize = 128;

    let mut handles = Vec::new();

    for _ in 0..NUM_THREADS {
        let state = Arc::clone(&state);

        handles.push(thread::spawn(move || {
            for _ in 0..NUM_ITERATIONS {
                state.apply(|value| value + 1).unwrap();
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(state.get().unwrap() as usize, NUM_THREADS * NUM_ITERATIONS);
}

#[test]
fn apply_boxed_slice_to_vec() {
    let state = OwnedState::<[u32]>::create_temporary().unwrap();
    state.set(&[0, 1]).unwrap();

    let result = state
        .apply_boxed(|slice| {
            let mut vec = slice.into_vec();
            vec.push(2);
            vec
        })
        .unwrap();

    assert_eq!(result, [0, 1, 2]);
    assert_eq!(*state.get_boxed().unwrap(), [0, 1, 2]);
}

#[test]
fn try_apply_boxed_slice_to_vec_ok() {
    let state = OwnedState::<[u32]>::create_temporary().unwrap();
    state.set(&[0, 1]).unwrap();

    let result = state
        .try_apply_boxed(|slice| {
            let mut vec = slice.into_vec();
            vec.push(2);
            Ok::<_, TestError>(vec)
        })
        .unwrap();

    assert_eq!(result, [0, 1, 2]);
    assert_eq!(*state.get_boxed().unwrap(), [0, 1, 2]);
}

#[test]
fn try_apply_boxed_slice_to_vec_err() {
    let state = OwnedState::<[u32]>::create_temporary().unwrap();
    state.set(&[0, 1]).unwrap();

    let result = state.try_apply_boxed(|_| Err::<Vec<_>, _>(TestError));

    assert!(matches!(result, Err(err) if err.kind() == ErrorKind::Other));
}

#[test]
fn apply_boxed_slice_to_vec_concurrent() {
    let state = Arc::new(OwnedState::<[u32]>::create_temporary().unwrap());

    const NUM_THREADS: usize = 2;
    const NUM_ITERATIONS: usize = 128;

    // This preemptively extends the internal capacity of the state to the maximum length,
    // avoiding concurrent reallocations, which can cause race conditions
    state
        .set(&(0..NUM_THREADS * NUM_ITERATIONS).map(|_| 0).collect::<Vec<_>>())
        .unwrap();
    state.set(&[]).unwrap();

    let mut handles = Vec::new();

    for _ in 0..NUM_THREADS {
        let state = Arc::clone(&state);

        handles.push(thread::spawn(move || {
            for _ in 0..NUM_ITERATIONS {
                state
                    .apply_boxed(|slice| {
                        let mut vec = slice.into_vec();
                        vec.push(0);
                        vec
                    })
                    .unwrap();
            }
        }))
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(state.get_boxed().unwrap().len(), NUM_THREADS * NUM_ITERATIONS);
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct TestError;

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for TestError {}
