use std::ffi::OsString;
use std::io::{stdin, Read};
use std::os::windows::ffi::OsStringExt;

use wnf::{BorrowedState, DataAccessor, SeenChangeStamp, StateListener, StateName, Subscription};

const WNF_SHEL_DESKTOP_APPLICATION_STARTED: u64 = 0x0D83063EA3BE5075;
const WNF_SHEL_DESKTOP_APPLICATION_TERMINATED: u64 = 0x0D83063EA3BE5875;

fn main() {
    println!("Listening to shell application starts and terminations, press any key to exit");

    let _subscription_start = subscribe(WNF_SHEL_DESKTOP_APPLICATION_STARTED, |change_stamp, application| {
        println!("Application start       #{change_stamp}: {application}")
    });

    let _subscription_termination = subscribe(WNF_SHEL_DESKTOP_APPLICATION_TERMINATED, |change_stamp, application| {
        println!("Application termination #{change_stamp}: {application}")
    });

    stdin().read(&mut [0u8]).unwrap();
}

fn subscribe<F>(state_name: impl Into<StateName>, listener: F) -> Subscription<'static, ApplicationListener<F>>
where
    F: FnMut(u32, &str) + Send + 'static,
{
    BorrowedState::from_state_name(state_name.into())
        .subscribe(ApplicationListener(listener), SeenChangeStamp::Current)
        .expect("Failed to subscribe to state changes")
}

struct ApplicationListener<F>(F);

impl<F> StateListener<[u16]> for ApplicationListener<F>
where
    F: FnMut(u32, &str) + Send + 'static,
{
    fn call(&mut self, accessor: DataAccessor<[u16]>) {
        let (data, change_stamp) = accessor
            .query_boxed()
            .expect("Failed to query state data")
            .into_data_change_stamp();

        if let Some(application) = OsString::from_wide(&data).to_string_lossy().strip_prefix("e:") {
            (self.0)(change_stamp.into(), application);
        }
    }
}
