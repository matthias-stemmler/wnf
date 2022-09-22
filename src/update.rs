use std::ffi::c_void;
use std::{io, mem, ptr};

use tracing::debug;
use windows::Win32::Foundation::{NTSTATUS, STATUS_UNSUCCESSFUL};

use crate::bytes::NoUninit;
use crate::data::WnfChangeStamp;
use crate::ntdll_sys::{self, NTDLL_TARGET};
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::state_name::WnfStateName;
use crate::type_id::TypeId;

impl<T> OwnedWnfState<T>
where
    T: NoUninit + ?Sized,
{
    pub fn set(&self, data: &T) -> io::Result<()> {
        self.raw.set(data)
    }

    pub fn update(&self, data: &T, expected_change_stamp: WnfChangeStamp) -> io::Result<bool> {
        self.raw.update(data, expected_change_stamp)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: NoUninit + ?Sized,
{
    pub fn set(self, data: &T) -> io::Result<()> {
        self.raw.set(data)
    }

    pub fn update(self, data: &T, expected_change_stamp: WnfChangeStamp) -> io::Result<bool> {
        self.raw.update(data, expected_change_stamp)
    }
}

impl<T> RawWnfState<T>
where
    T: NoUninit + ?Sized,
{
    fn set(self, data: &T) -> io::Result<()> {
        update(
            self.state_name,
            self.type_id,
            data as *const T as *const c_void,
            mem::size_of_val(data),
            None,
        )
        .ok()?;
        Ok(())
    }

    pub(crate) fn update(self, data: &T, expected_change_stamp: WnfChangeStamp) -> io::Result<bool> {
        let result = update(
            self.state_name,
            self.type_id,
            data as *const T as *const c_void,
            mem::size_of_val(data),
            Some(expected_change_stamp),
        );

        Ok(if result == STATUS_UNSUCCESSFUL {
            false
        } else {
            result.ok()?;
            true
        })
    }
}

fn update(
    state_name: WnfStateName,
    type_id: TypeId,
    buffer: *const c_void,
    buffer_size: usize,
    expected_change_stamp: Option<WnfChangeStamp>,
) -> NTSTATUS {
    let matching_change_stamp = expected_change_stamp.unwrap_or_default().into();
    let check_stamp = expected_change_stamp.is_some() as u32;

    let result = unsafe {
        ntdll_sys::ZwUpdateWnfStateData(
            &state_name.opaque_value(),
            buffer,
            buffer_size as u32,
            type_id.as_ptr(),
            ptr::null(),
            matching_change_stamp,
            check_stamp,
        )
    };

    debug!(
        target: NTDLL_TARGET,
        ?result,
        input.state_name = %state_name,
        input.buffer_size = buffer_size,
        input.type_id = %type_id,
        input.matching_change_stamp = matching_change_stamp,
        input.check_stamp = check_stamp,
        "ZwUpdateWnfStateData",
    );

    result
}
