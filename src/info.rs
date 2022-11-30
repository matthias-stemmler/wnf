//! Methods for obtaining information on states

use std::ffi::c_void;
use std::{io, mem, ptr};

use tracing::debug;

use crate::ntapi;
use crate::state::{BorrowedState, OwnedState, RawState};

/// Different classes of information on a state
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
enum NameInfoClass {
    /// Whether a state with a given name exists
    StateNameExist = 0,

    /// Whether a state has at least one subscriber
    SubscribersPresent = 1,

    /// Whether a state is "quiescent", i.e. none of the listeners subscribed to it are currently running
    IsQuiescent = 2,
}

impl<T> OwnedState<T>
where
    T: ?Sized,
{
    /// Returns whether this state exists
    ///
    /// # Errors
    /// Returns an error if obtaining the information fails
    pub fn exists(&self) -> io::Result<bool> {
        self.raw.exists()
    }

    /// Returns whether this state has at least one subscriber
    ///
    /// # Errors
    /// Returns an error if obtaining the information fails
    pub fn subscribers_present(&self) -> io::Result<bool> {
        self.raw.subscribers_present()
    }

    /// Returns whether this state is "quiescent", i.e. none of the listeners subscribed to it are currently running
    ///
    /// # Errors
    /// Returns an error if obtaining the information fails
    pub fn is_quiescent(&self) -> io::Result<bool> {
        self.raw.is_quiescent()
    }
}

impl<T> BorrowedState<'_, T>
where
    T: ?Sized,
{
    /// Returns whether this state exists
    ///
    /// See [`OwnedState::exists`]
    pub fn exists(self) -> io::Result<bool> {
        self.raw.exists()
    }

    /// Returns whether this state has at least one subscriber
    ///
    /// See [`OwnedState::subscribers_present`]
    pub fn subscribers_present(self) -> io::Result<bool> {
        self.raw.subscribers_present()
    }

    /// Returns whether this state is "quiescent", i.e. none of the listeners subscribed to it are currently running
    ///
    /// See [`OwnedState::is_quiescent`]
    pub fn is_quiescent(self) -> io::Result<bool> {
        self.raw.is_quiescent()
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    /// Returns whether this state exists
    fn exists(self) -> io::Result<bool> {
        self.info_internal(NameInfoClass::StateNameExist)
    }

    /// Returns whether this state has at least one subscriber
    fn subscribers_present(self) -> io::Result<bool> {
        self.info_internal(NameInfoClass::SubscribersPresent)
    }

    /// Returns whether this state is "quiescent", i.e. none of the listeners subscribed to it are currently running
    fn is_quiescent(self) -> io::Result<bool> {
        self.info_internal(NameInfoClass::IsQuiescent)
    }

    /// Returns the flag containing the information of the given class
    fn info_internal(self, name_info_class: NameInfoClass) -> io::Result<bool> {
        let mut buffer = u32::MAX;
        let name_info_class = name_info_class as u32;

        // SAFETY:
        // - The pointer in the first argument points to a valid `u64` because it comes from a live reference
        // - The pointer in the fifth argument is valid for writes of `u32` because it comes from a live mutable
        //   reference
        // - The number in the sixth argument is `4` because it equals `mem::size_of::<u32>()`
        let result = unsafe {
            ntapi::NtQueryWnfStateNameInformation(
                &self.state_name.opaque_value(),
                name_info_class,
                ptr::null(),
                &mut buffer as *mut u32 as *mut c_void,
                mem::size_of_val(&buffer) as u32,
            )
        };

        if result.is_ok() {
            debug!(
                 target: ntapi::TRACING_TARGET,
                 ?result,
                 input.state_name = %self.state_name,
                 input.name_info_class = name_info_class,
                 output.buffer = buffer,
                 "NtQueryWnfStateNameInformation",
            );

            Ok(match buffer {
                0 => false,
                1 => true,
                _ => unreachable!("NtQueryWnfStateNameInformation did not produce valid boolean"),
            })
        } else {
            debug!(
                 target: ntapi::TRACING_TARGET,
                 ?result,
                 input.state_name = %self.state_name,
                 input.name_info_class = name_info_class,
                 "NtQueryWnfStateNameInformation",
            );

            Err(io::Error::from_raw_os_error(result.0))
        }
    }
}
