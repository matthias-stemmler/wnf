use std::ptr;

use thiserror::Error;
use tracing::debug;
use windows::Win32::Foundation::NTSTATUS;

use crate::ntdll::NTDLL_TARGET;
use crate::ntdll_sys;
use crate::security::{SecurityCreateError, SecurityDescriptor};
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::state_name::{WnfDataScope, WnfStateName, WnfStateNameLifetime};

impl<T> OwnedWnfState<T> {
    pub fn create_temporary() -> Result<Self, WnfCreateError> {
        RawWnfState::create_temporary().map(Self::from_raw)
    }

    pub fn delete(self) -> Result<(), WnfDeleteError> {
        self.into_raw().delete()
    }
}

impl<'a, T> BorrowedWnfState<'a, T> {
    pub fn delete(self) -> Result<(), WnfDeleteError> {
        self.into_raw().delete()
    }
}

impl<T> RawWnfState<T> {
    pub(crate) fn create_temporary() -> Result<Self, WnfCreateError> {
        let mut opaque_value = 0;

        // TODO Can we drop this or is it "borrowed" by the created WNF state?
        let security_descriptor = SecurityDescriptor::create_everyone_generic_all()?;

        let name_lifetime = WnfStateNameLifetime::Temporary as u32;
        let data_scope = WnfDataScope::Machine as u32;
        let persist_data = 0;
        let maximum_state_size = 0x1000;

        let result = unsafe {
            ntdll_sys::ZwCreateWnfStateName(
                &mut opaque_value,
                name_lifetime,
                data_scope,
                persist_data,
                ptr::null(),
                maximum_state_size,
                security_descriptor.as_void_ptr(),
            )
        };

        if result.is_ok() {
            let state_name = WnfStateName::from_opaque_value(opaque_value);

            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.name_lifetime = name_lifetime,
                input.data_scope = data_scope,
                input.persist_data = persist_data,
                input.maximum_state_size = maximum_state_size,
                output.state_name = %state_name,
                "ZwCreateWnfStateName",
            );

            Ok(Self::from_state_name(state_name))
        } else {
            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.name_lifetime = name_lifetime,
                input.data_scope = data_scope,
                input.persist_data = persist_data,
                input.maximum_state_size = maximum_state_size,
                "ZwCreateWnfStateName",
            );

            Err(result.into())
        }
    }

    pub(crate) fn delete(self) -> Result<(), WnfDeleteError> {
        let result = unsafe { ntdll_sys::ZwDeleteWnfStateName(&self.state_name.opaque_value()) };

        debug!(
            target: NTDLL_TARGET,
            ?result,
            input.state_name = %self.state_name,
            "ZwDeleteWnfStateName",
        );

        result.ok()?;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfCreateError {
    #[error("failed to create WNF state name: security error {0}")]
    Security(#[from] SecurityCreateError),

    #[error("failed to create WNF state name: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

impl From<NTSTATUS> for WnfCreateError {
    fn from(result: NTSTATUS) -> Self {
        let err: windows::core::Error = result.into();
        err.into()
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfDeleteError {
    #[error("failed to delete WNF state name: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

impl From<NTSTATUS> for WnfDeleteError {
    fn from(result: NTSTATUS) -> Self {
        let err: windows::core::Error = result.into();
        err.into()
    }
}
