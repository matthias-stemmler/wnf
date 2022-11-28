use std::sync::Arc;
use std::thread;

use wnf::OwnedState;

#[test]
fn replace() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    state.set(&0).unwrap();

    let previous_value = state.replace(&1).unwrap();

    assert_eq!(state.get().unwrap(), 1);
    assert_eq!(previous_value, 0);
}

#[test]
fn replace_concurrent() {
    let state = Arc::new(OwnedState::<u32>::create_temporary().unwrap());
    state.set(&0).unwrap();

    const NUM_THREADS: usize = 2;
    const NUM_ITERATIONS: usize = 128;

    let mut handles = Vec::new();

    for i in 0..NUM_THREADS {
        let state = Arc::clone(&state);

        handles.push(thread::spawn(move || {
            let mut previous_values = Vec::new();

            for j in 0..NUM_ITERATIONS {
                let value = (1 + i * NUM_ITERATIONS + j) as u32;
                let previous_value = state.replace(&value).unwrap();
                previous_values.push(previous_value);
            }

            previous_values
        }));
    }

    let mut values: Vec<u32> = handles
        .into_iter()
        .flat_map(|handle| handle.join().unwrap().into_iter())
        .collect();

    let final_value = state.get().unwrap();

    values.push(final_value);
    values.sort();

    assert_eq!(values, (0..=(NUM_THREADS * NUM_ITERATIONS) as u32).collect::<Vec<_>>());
}

#[test]
fn replace_boxed_slice() {
    let state = OwnedState::<[u32]>::create_temporary().unwrap();
    state.set(&[0]).unwrap();

    let previous_slice = state.replace_boxed(&[0, 1]).unwrap();

    assert_eq!(*state.get_boxed().unwrap(), [0, 1]);
    assert_eq!(*previous_slice, [0]);
}

#[test]
fn replace_boxed_slice_concurrent() {
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

    for i in 0..NUM_THREADS {
        let state = Arc::clone(&state);

        handles.push(thread::spawn(move || {
            let mut previous_lengths = Vec::new();

            for j in 0..NUM_ITERATIONS {
                let slice: Vec<_> = (0..(1 + i * NUM_ITERATIONS + j)).map(|_| 0).collect();
                let previous_slice = state.replace_boxed(&slice).unwrap();
                previous_lengths.push(previous_slice.len());
            }

            previous_lengths
        }));
    }

    let mut lengths: Vec<usize> = handles
        .into_iter()
        .flat_map(|handle| handle.join().unwrap().into_iter())
        .collect();

    let final_length = state.get_boxed().unwrap().len();

    lengths.push(final_length);
    lengths.sort();

    assert_eq!(lengths, (0..=NUM_THREADS * NUM_ITERATIONS).collect::<Vec<_>>());
}
