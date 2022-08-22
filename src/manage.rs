use std::{io, ptr};

use tracing::debug;

use crate::ntdll::NTDLL_TARGET;
use crate::ntdll_sys;
use crate::security::SecurityDescriptor;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::state_name::{WnfDataScope, WnfStateName, WnfStateNameLifetime};

impl<T> OwnedWnfState<T>
where
    T: ?Sized,
{
    pub fn create_temporary() -> io::Result<Self> {
        RawWnfState::create_temporary().map(Self::from_raw)
    }

    pub fn delete(self) -> io::Result<()> {
        self.into_raw().delete()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: ?Sized,
{
    pub fn delete(self) -> io::Result<()> {
        self.raw.delete()
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    pub(crate) fn create_temporary() -> io::Result<Self> {
        let mut opaque_value = 0;

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

            Err(io::Error::from_raw_os_error(result.0))
        }
    }

    pub(crate) fn delete(self) -> io::Result<()> {
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
