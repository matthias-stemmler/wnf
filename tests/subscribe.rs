use std::{
    sync::mpsc::{self, RecvTimeoutError},
    time::Duration,
};

use wnf::{OwnedWnfState, WnfChangeStamp, WnfDataAccessor, WnfSeenChangeStamp};

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
                .subscribe(
                    move |accessor: WnfDataAccessor<_>| {
                        tx.send(accessor.query().unwrap()).unwrap();
                    },
                    WnfSeenChangeStamp::None,
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
            assert_eq!(change_stamp, WnfChangeStamp::from(i));
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
                .subscribe(
                    move |accessor: WnfDataAccessor<_>| {
                        tx.send(accessor.query_boxed().unwrap()).unwrap();
                    },
                    WnfSeenChangeStamp::None,
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
            assert_eq!(change_stamp, WnfChangeStamp::from(i));
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
                .subscribe(
                    move |accessor: WnfDataAccessor<_>| {
                        tx.send(accessor.query_boxed().unwrap()).unwrap();
                    },
                    WnfSeenChangeStamp::Current,
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
            assert_eq!(change_stamp, WnfChangeStamp::from(i));
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
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();
    state.set(&0).unwrap();

    let (tx, rx) = mpsc::channel();

    let subscription = state
        .subscribe(
            move |accessor: WnfDataAccessor<_>| {
                tx.send(accessor.change_stamp()).unwrap();
            },
            WnfSeenChangeStamp::None,
        )
        .unwrap();

    let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(change_stamp, WnfChangeStamp::from(1));

    for i in 1..3 {
        state.set(&i).unwrap();
        let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(change_stamp, WnfChangeStamp::from(i + 1));
    }

    subscription.unsubscribe().unwrap();

    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1)),
        Err(RecvTimeoutError::Disconnected)
    );
}

#[test]
fn subscribe_with_last_seen_change_stamp_current() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();
    state.set(&0).unwrap();

    let (tx, rx) = mpsc::channel();

    let subscription = state
        .subscribe(
            move |accessor: WnfDataAccessor<_>| {
                tx.send(accessor.change_stamp()).unwrap();
            },
            WnfSeenChangeStamp::Current,
        )
        .unwrap();

    for i in 1..3 {
        state.set(&i).unwrap();
        let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(change_stamp, WnfChangeStamp::from(i + 1));
    }

    subscription.unsubscribe().unwrap();

    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1)),
        Err(RecvTimeoutError::Disconnected)
    );
}

#[test]
fn subscribe_with_last_seen_change_stamp_value() {
    let state = OwnedWnfState::<u32>::create_temporary().unwrap();
    state.set(&0).unwrap();

    let (tx, rx) = mpsc::channel();

    let subscription = state
        .subscribe(
            move |accessor: WnfDataAccessor<_>| {
                tx.send(accessor.change_stamp()).unwrap();
            },
            WnfSeenChangeStamp::Value(WnfChangeStamp::from(2)),
        )
        .unwrap();

    for i in 1..3 {
        state.set(&i).unwrap();
    }

    let change_stamp = rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(change_stamp, WnfChangeStamp::from(3));

    subscription.unsubscribe().unwrap();

    assert_eq!(
        rx.recv_timeout(Duration::from_secs(1)),
        Err(RecvTimeoutError::Disconnected)
    );
}
