use std::ffi::c_void;
use std::{mem, ptr};

use thiserror::Error;
use tracing::debug;
use windows::Win32::Foundation::NTSTATUS;

use crate::ntdll::NTDLL_TARGET;
use crate::ntdll_sys;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
pub(crate) enum WnfNameInfoClass {
    StateNameExist = 0,
    SubscribersPresent = 1,
    IsQuiescent = 2,
}

impl<T> OwnedWnfState<T> {
    pub fn exists(&self) -> Result<bool, WnfInfoError> {
        self.raw.exists()
    }

    pub fn subscribers_present(&self) -> Result<bool, WnfInfoError> {
        self.raw.subscribers_present()
    }

    pub fn is_quiescent(&self) -> Result<bool, WnfInfoError> {
        self.raw.is_quiescent()
    }
}

impl<T> BorrowedWnfState<'_, T> {
    pub fn exists(&self) -> Result<bool, WnfInfoError> {
        self.raw.exists()
    }

    pub fn subscribers_present(&self) -> Result<bool, WnfInfoError> {
        self.raw.subscribers_present()
    }

    pub fn is_quiescent(&self) -> Result<bool, WnfInfoError> {
        self.raw.is_quiescent()
    }
}

impl<T> RawWnfState<T> {
    pub fn exists(&self) -> Result<bool, WnfInfoError> {
        self.info_internal(WnfNameInfoClass::StateNameExist)
    }

    pub fn subscribers_present(&self) -> Result<bool, WnfInfoError> {
        self.info_internal(WnfNameInfoClass::SubscribersPresent)
    }

    pub fn is_quiescent(&self) -> Result<bool, WnfInfoError> {
        self.info_internal(WnfNameInfoClass::IsQuiescent)
    }

    fn info_internal(&self, name_info_class: WnfNameInfoClass) -> Result<bool, WnfInfoError> {
        let mut buffer = u32::MAX;
        let name_info_class = name_info_class as u32;

        let result = unsafe {
            ntdll_sys::ZwQueryWnfStateNameInformation(
                &self.state_name.opaque_value(),
                name_info_class,
                ptr::null(),
                &mut buffer as *mut _ as *mut c_void,
                mem::size_of_val(&buffer) as u32,
            )
        };

        if result.is_ok() {
            debug!(
                 target: NTDLL_TARGET,
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
                 target: NTDLL_TARGET,
                 ?result,
                 input.state_name = %self.state_name,
                 input.name_info_class = name_info_class,
                 "ZwQueryWnfStateNameInformation",
            );

            Err(result.into())
        }
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfInfoError {
    #[error("failed to query WNF state information: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

impl From<NTSTATUS> for WnfInfoError {
    fn from(result: NTSTATUS) -> Self {
        let err: windows::core::Error = result.into();
        err.into()
    }
}
