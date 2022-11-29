use std::time::Duration;

use crossbeam_channel::RecvTimeoutError;
use wnf::{AsState, ChangeStamp, DataAccessor, OpaqueData, OwnedState, SeenChangeStamp};

#[test]
fn subscribe() {
    let state = OwnedState::<u32>::create_temporary().unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();
    let mut subscriptions = Vec::new();

    const NUM_SUBSCRIPTIONS: usize = 2;

    for _ in 0..NUM_SUBSCRIPTIONS {
        let tx = tx.clone();
        subscriptions.push(
            state
                .subscribe(
                    move |accessor: DataAccessor<_>| {
                        tx.send(accessor.query().unwrap()).unwrap();
                    },
                    SeenChangeStamp::None,
                )
                .unwrap(),
        )
    }

    drop(tx);

    for i in 1..3 {
        state.set(&i).unwrap();

        for _ in 0..NUM_SUBSCRIPTIONS {
            let (data, change_stamp) = rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .into_data_change_stamp();

            assert_eq!(data, i);
            assert_eq!(change_stamp, ChangeStamp::from(i));
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
    let state = OwnedState::<u32>::create_temporary().unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();
    let mut subscriptions = Vec::new();

    const NUM_SUBSCRIPTIONS: usize = 2;

    for _ in 0..NUM_SUBSCRIPTIONS {
        let tx = tx.clone();
        subscriptions.push(
            state
                .subscribe(
                    move |accessor: DataAccessor<_>| {
                        tx.send(accessor.query_boxed().unwrap()).unwrap();
                    },
                    SeenChangeStamp::None,
                )
                .unwrap(),
        )
    }

    drop(tx);

    for i in 1..3 {
        state.set(&i).unwrap();

        for _ in 0..NUM_SUBSCRIPTIONS {
            let (data, change_stamp) = rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .into_data_change_stamp();

            assert_eq!(*data, i);
            assert_eq!(change_stamp, ChangeStamp::from(i));
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
    let state = OwnedState::<[u32]>::create_temporary().unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();
    let mut subscriptions = Vec::new();

    const NUM_SUBSCRIPTIONS: usize = 2;

    for _ in 0..NUM_SUBSCRIPTIONS {
        let tx = tx.clone();
        subscriptions.push(
            state
                .subscribe(
                    move |accessor: DataAccessor<_>| {
                        tx.send(accessor.query_boxed().unwrap()).unwrap();
                    },
                    SeenChangeStamp::Current,
                )
                .unwrap(),
        )
    }

    drop(tx);

    for i in 1..3 {
        state.set(&[i, i]).unwrap();

        for _ in 0..NUM_SUBSCRIPTIONS {
            let (data, change_stamp) = rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .into_data_change_stamp();

            assert_eq!(*data, [i, i]);
            assert_eq!(change_stamp, ChangeStamp::from(i));
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
fn subscribe_with_last_seen_change_stamp_none() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    state.set(&0).unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();

    let subscription = state
        .subscribe(
            move |accessor: DataAccessor<_>| {
                tx.send(accessor.change_stamp()).unwrap();
            },
            SeenChangeStamp::None,
        )
        .unwrap();

    let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(change_stamp, ChangeStamp::from(1));

    for i in 1..3 {
        state.set(&i).unwrap();
        let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(change_stamp, ChangeStamp::from(i + 1));
    }

    subscription.unsubscribe().unwrap();

    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1)),
        Err(RecvTimeoutError::Disconnected)
    );
}

#[test]
fn subscribe_with_last_seen_change_stamp_current() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    state.set(&0).unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();

    let subscription = state
        .subscribe(
            move |accessor: DataAccessor<_>| {
                tx.send(accessor.change_stamp()).unwrap();
            },
            SeenChangeStamp::Current,
        )
        .unwrap();

    for i in 1..3 {
        state.set(&i).unwrap();
        let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(change_stamp, ChangeStamp::from(i + 1));
    }

    subscription.unsubscribe().unwrap();

    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1)),
        Err(RecvTimeoutError::Disconnected)
    );
}

#[test]
fn subscribe_with_last_seen_change_stamp_value() {
    let state = OwnedState::<u32>::create_temporary().unwrap();
    state.set(&0).unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();

    let subscription = state
        .subscribe(
            move |accessor: DataAccessor<_>| {
                tx.send(accessor.change_stamp()).unwrap();
            },
            SeenChangeStamp::Value(ChangeStamp::from(2)),
        )
        .unwrap();

    for i in 1..3 {
        state.set(&i).unwrap();
    }

    let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(change_stamp, ChangeStamp::from(3));

    subscription.unsubscribe().unwrap();

    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1)),
        Err(RecvTimeoutError::Disconnected)
    );
}

#[test]
fn subscribe_opaque_data() {
    let state = OwnedState::<OpaqueData>::create_temporary().unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();

    let _subscription = state.subscribe(
        move |accessor: DataAccessor<_>| {
            tx.send(accessor.change_stamp()).unwrap();
        },
        SeenChangeStamp::None,
    );

    state.as_state().cast::<u32>().set(&42).unwrap();
    let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(change_stamp, 1);

    state.as_state().cast::<u16>().set(&43).unwrap();
    let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(change_stamp, 2);
}
