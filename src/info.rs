//! Methods for obtaining information on WNF states

use std::ffi::c_void;
use std::{io, mem, ptr};

use tracing::debug;

use crate::ntapi;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};

/// Different classes of information on a WNF state
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
enum WnfNameInfoClass {
    /// Whether a state with a given name exists
    StateNameExist = 0,

    /// Whether a state has at least one subscriber
    SubscribersPresent = 1,

    /// Whether a state is "quiescent", i.e. none of the listeners subscribed to it are currently running
    IsQuiescent = 2,
}

impl<T> OwnedWnfState<T>
where
    T: ?Sized,
{
    /// Returns whether a WNF state with the name represented by this [`OwnedWnfState<T>`] exists
    pub fn exists(&self) -> io::Result<bool> {
        self.raw.exists()
    }

    /// Returns whether this [`OwnedWnfState<T>`] has at least one subscriber
    pub fn subscribers_present(&self) -> io::Result<bool> {
        self.raw.subscribers_present()
    }

    /// Returns whether this [`OwnedWnfState<T>`] is "quiescent", i.e. none of the listeners subscribed to it are
    /// currently running
    pub fn is_quiescent(&self) -> io::Result<bool> {
        self.raw.is_quiescent()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: ?Sized,
{
    /// Returns whether a WNF state with the name represented by this [`BorrowedWnfState<'a, T>`] exists
    pub fn exists(self) -> io::Result<bool> {
        self.raw.exists()
    }

    /// Returns whether this [`BorrowedWnfState<'a, T>`] has at least one subscriber
    pub fn subscribers_present(self) -> io::Result<bool> {
        self.raw.subscribers_present()
    }

    /// Returns whether this [`BorrowedWnfState<'a, T>`] is "quiescent", i.e. none of the listeners subscribed to it are
    /// currently running
    pub fn is_quiescent(self) -> io::Result<bool> {
        self.raw.is_quiescent()
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    /// Returns whether a WNF state with the name represented by this [`RawWnfState<T>`] exists
    fn exists(self) -> io::Result<bool> {
        self.info_internal(WnfNameInfoClass::StateNameExist)
    }

    /// Returns whether this [`RawWnfState<T>`] has at least one subscriber
    fn subscribers_present(self) -> io::Result<bool> {
        self.info_internal(WnfNameInfoClass::SubscribersPresent)
    }

    /// Returns whether this [`RawWnfState<T>`] is "quiescent", i.e. none of the listeners subscribed to it are
    /// currently running
    fn is_quiescent(self) -> io::Result<bool> {
        self.info_internal(WnfNameInfoClass::IsQuiescent)
    }

    /// Returns the flag containing the information of the given class
    fn info_internal(self, name_info_class: WnfNameInfoClass) -> io::Result<bool> {
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
