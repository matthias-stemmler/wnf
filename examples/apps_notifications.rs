//! Manipulating and waiting on notifications

use std::error::Error;
use std::time::Duration;

use tokio::io::{stdin, AsyncReadExt};
use tokio::time;
use wnf::{BorrowedState, StateName};

const WNF_SHEL_NOTIFICATIONS: StateName = StateName::from_opaque_value(0x0D83063EA3BC1035);

const NUM_NOTIFICATIONS: u32 = 42;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let state = BorrowedState::<u32>::from_state_name(WNF_SHEL_NOTIFICATIONS);

    println!("Look at your notifications, then press ENTER ...");

    stdin().read_u8().await?;

    for i in 0..=NUM_NOTIFICATIONS {
        state.set(&i)?;
        time::sleep(Duration::from_millis(30)).await;
    }

    println!("Now you have {NUM_NOTIFICATIONS} unread notifications. Please read them!");

    state.wait_until_async(|value| *value == 0).await?;

    println!("Thanks, bye");

    Ok(())
}
