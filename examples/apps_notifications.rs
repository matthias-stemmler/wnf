//! Manipulating and waiting on notifications

use std::time::Duration;

use tokio::io::{stdin, AsyncReadExt};
use tokio::time;
use wnf::BorrowedState;

const WNF_SHEL_NOTIFICATIONS: u64 = 0x0D83063EA3BC1035;

const NUM_NOTIFICATIONS: u32 = 42;

#[tokio::main]
async fn main() {
    let state = BorrowedState::<u32>::from_state_name(WNF_SHEL_NOTIFICATIONS.into());

    println!("Look at your notifications, then press ENTER ...");

    stdin().read_u8().await.unwrap();

    for i in 0..=NUM_NOTIFICATIONS {
        state.set(&i).expect("Failed to set state data");
        time::sleep(Duration::from_millis(30)).await;
    }

    println!("Now you have {NUM_NOTIFICATIONS} unread notifications. Please read them!");

    state
        .wait_until_async(|value| *value == 0)
        .await
        .expect("Failed to wait for state update");

    println!("Thanks, bye");
}
