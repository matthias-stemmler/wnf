use std::ffi::c_void;
use std::{io, mem, ptr};

use tracing::debug;

use crate::ntdll;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
enum WnfNameInfoClass {
    StateNameExist = 0,
    SubscribersPresent = 1,
    IsQuiescent = 2,
}

impl<T> OwnedWnfState<T>
where
    T: ?Sized,
{
    pub fn exists(&self) -> io::Result<bool> {
        self.raw.exists()
    }

    pub fn subscribers_present(&self) -> io::Result<bool> {
        self.raw.subscribers_present()
    }

    pub fn is_quiescent(&self) -> io::Result<bool> {
        self.raw.is_quiescent()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: ?Sized,
{
    pub fn exists(self) -> io::Result<bool> {
        self.raw.exists()
    }

    pub fn subscribers_present(self) -> io::Result<bool> {
        self.raw.subscribers_present()
    }

    pub fn is_quiescent(self) -> io::Result<bool> {
        self.raw.is_quiescent()
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    fn exists(self) -> io::Result<bool> {
        self.info_internal(WnfNameInfoClass::StateNameExist)
    }

    fn subscribers_present(self) -> io::Result<bool> {
        self.info_internal(WnfNameInfoClass::SubscribersPresent)
    }

    fn is_quiescent(self) -> io::Result<bool> {
        self.info_internal(WnfNameInfoClass::IsQuiescent)
    }

    fn info_internal(self, name_info_class: WnfNameInfoClass) -> io::Result<bool> {
        let mut buffer = u32::MAX;
        let name_info_class = name_info_class as u32;

        let result = unsafe {
            ntdll::ZwQueryWnfStateNameInformation(
                &self.state_name.opaque_value(),
                name_info_class,
                ptr::null(),
                &mut buffer as *mut u32 as *mut c_void,
                mem::size_of_val(&buffer) as u32,
            )
        };

        if result.is_ok() {
            debug!(
                 target: ntdll::TRACING_TARGET,
                 ?result,
                 input.state_name = %self.state_name,
                 input.name_info_class = name_info_class,
                 output.buffer = buffer,
                 "ZwQueryWnfStateNameInformation",
            );

            Ok(match buffer {
                0 => false,
                1 => true,
                _ => unreachable!("ZwQueryWnfStateNameInformation did not produce valid boolean"),
            })
        } else {
            debug!(
                 target: ntdll::TRACING_TARGET,
                 ?result,
                 input.state_name = %self.state_name,
                 input.name_info_class = name_info_class,
                 "ZwQueryWnfStateNameInformation",
            );

            Err(io::Error::from_raw_os_error(result.0))
        }
    }
}
