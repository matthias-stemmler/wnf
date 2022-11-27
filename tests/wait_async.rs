use std::sync::Arc;
use std::time::Duration;

use tokio::time;
use wnf::OwnedState;

#[tokio::test]
async fn wait_async() {
    let state = Arc::new(OwnedState::<u32>::create_temporary().unwrap());

    let (tx, rx) = async_channel::unbounded();

    let handle = {
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            for _ in 0..2 {
                time::timeout(Duration::from_secs(3), state.wait_async())
                    .await
                    .unwrap()
                    .unwrap();
                tx.send(state.change_stamp().unwrap()).await.unwrap();
            }
        })
    };

    time::sleep(Duration::from_millis(300)).await;
    state.set(&42).unwrap();
    let change_stamp = time::timeout(Duration::from_secs(1), rx.recv()).await.unwrap().unwrap();
    assert_eq!(change_stamp, 1.into());

    time::sleep(Duration::from_millis(300)).await;
    state.set(&43).unwrap();
    let change_stamp = time::timeout(Duration::from_secs(1), rx.recv()).await.unwrap().unwrap();
    assert_eq!(change_stamp, 2.into());

    handle.await.unwrap();
}

#[tokio::test]
async fn wait_until_async() {
    let state = Arc::new(OwnedState::<u32>::create_temporary().unwrap());
    state.set(&0).unwrap();

    let (tx, rx) = async_channel::unbounded();

    let handle = {
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            let value = time::timeout(Duration::from_secs(3), state.wait_until_async(|value| *value > 42))
                .await
                .unwrap()
                .unwrap();

            tx.send(value).await.unwrap();
        })
    };

    time::sleep(Duration::from_millis(300)).await;
    state.set(&42).unwrap();

    time::sleep(Duration::from_millis(300)).await;
    state.set(&43).unwrap();

    let value = time::timeout(Duration::from_secs(1), rx.recv()).await.unwrap().unwrap();
    assert_eq!(value, 43);

    handle.await.unwrap();
}

#[tokio::test]
async fn wait_until_boxed_async() {
    let state = Arc::new(OwnedState::<[u32]>::create_temporary().unwrap());
    state.set(&[]).unwrap();

    let (tx, rx) = async_channel::unbounded();

    let handle = {
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            let value = time::timeout(
                Duration::from_secs(3),
                state.wait_until_boxed_async(|slice| slice.len() > 1),
            )
            .await
            .unwrap()
            .unwrap();

            tx.send(value).await.unwrap();
        })
    };

    time::sleep(Duration::from_millis(300)).await;
    state.set(&[0]).unwrap();

    time::sleep(Duration::from_millis(300)).await;
    state.set(&[0, 1]).unwrap();

    let value = time::timeout(Duration::from_secs(1), rx.recv()).await.unwrap().unwrap();
    assert_eq!(*value, [0, 1]);

    handle.await.unwrap();
}
