use std::io::ErrorKind;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use wnf::OwnedState;

#[test]
fn wait_blocking() {
    let state = Arc::new(OwnedState::<u32>::create_temporary().unwrap());

    let (tx, rx) = crossbeam_channel::unbounded();

    let handle = {
        let state = Arc::clone(&state);

        thread::spawn(move || {
            for _ in 0..2 {
                state.wait_blocking(Duration::from_secs(3)).unwrap();
                tx.send(state.change_stamp().unwrap()).unwrap();
            }
        })
    };

    thread::sleep(Duration::from_millis(300));
    state.set(&42).unwrap();
    let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(change_stamp, 1);

    thread::sleep(Duration::from_millis(300));
    state.set(&43).unwrap();
    let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(change_stamp, 2);

    handle.join().unwrap();
}

#[test]
fn wait_blocking_timeout() {
    let state = OwnedState::<u32>::create_temporary().unwrap();

    let result = state.wait_blocking(Duration::ZERO);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::TimedOut);
}

#[test]
fn wait_until_blocking() {
    let state = Arc::new(OwnedState::<u32>::create_temporary().unwrap());
    state.set(&0).unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();

    let handle = {
        let state = Arc::clone(&state);

        thread::spawn(move || {
            let value = state
                .wait_until_blocking(|value| *value > 42, Duration::from_secs(3))
                .unwrap();
            tx.send(value).unwrap();
        })
    };

    thread::sleep(Duration::from_millis(300));
    state.set(&42).unwrap();

    thread::sleep(Duration::from_millis(300));
    state.set(&43).unwrap();

    let value = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(value, 43);

    handle.join().unwrap();
}

#[test]
fn wait_until_blocking_timeout() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    state.set(&0).unwrap();

    let result = state.wait_until_blocking(|_| false, Duration::ZERO);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::TimedOut);
}

#[test]
fn wait_until_boxed_blocking() {
    let state = Arc::new(OwnedState::<[u32]>::create_temporary().unwrap());
    state.set(&[]).unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();

    let handle = {
        let state = Arc::clone(&state);

        thread::spawn(move || {
            let value = state
                .wait_until_boxed_blocking(|slice| slice.len() > 1, Duration::from_secs(3))
                .unwrap();
            tx.send(value).unwrap();
        })
    };

    thread::sleep(Duration::from_millis(300));
    state.set(&[0]).unwrap();

    thread::sleep(Duration::from_millis(300));
    state.set(&[0, 1]).unwrap();

    let value = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(*value, [0, 1]);

    handle.join().unwrap();
}

#[test]
fn wait_until_boxed_blocking_timeout() {
    let state = OwnedState::<[u32]>::create_temporary().unwrap();
    state.set(&[]).unwrap();

    let result = state.wait_until_boxed_blocking(|_| false, Duration::ZERO);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::TimedOut);
}
