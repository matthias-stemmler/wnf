use std::ffi::OsString;
use std::io::{stdin, Read};
use std::os::windows::ffi::OsStringExt;

use wnf::{BorrowedWnfState, WnfDataAccessor, WnfStateListener, WnfStateName, WnfSubscription};

const WNF_SHEL_DESKTOP_APPLICATION_STARTED: WnfStateName = WnfStateName::from_opaque_value(0xd83063ea3be5075);
const WNF_SHEL_DESKTOP_APPLICATION_TERMINATED: WnfStateName = WnfStateName::from_opaque_value(0xd83063ea3be5875);

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

fn subscribe<F>(state_name: WnfStateName, listener: F) -> WnfSubscription<'static, ApplicationListener<F>>
where
    F: FnMut(u32, &str) + Send + 'static,
{
    let state = BorrowedWnfState::from_state_name(state_name);
    let change_stamp = state.change_stamp().expect("Failed to get WNF state change stamp");

    state
        .subscribe(change_stamp, ApplicationListener(listener))
        .expect("Failed to subscribe to WNF state changes")
}

struct ApplicationListener<F>(F);

impl<F> WnfStateListener<[u16]> for ApplicationListener<F>
where
    F: FnMut(u32, &str) + Send + 'static,
{
    fn call(&mut self, accessor: WnfDataAccessor<[u16]>) {
        let (data, change_stamp) = accessor
            .query_boxed()
            .expect("Failed to query WNF state data")
            .into_data_change_stamp();

        if let Some(application) = OsString::from_wide(&data).to_string_lossy().strip_prefix("e:") {
            (self.0)(change_stamp.into(), application);
        }
    }
}
