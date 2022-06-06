use std::borrow::Borrow;
use std::ptr;

use thiserror::Error;
use tracing::debug;
use windows::Win32::Foundation::{NTSTATUS, STATUS_UNSUCCESSFUL};

use crate::data::WnfChangeStamp;
use crate::ntdll::NTDLL_TARGET;
use crate::ntdll_sys;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::write::WnfWrite;

impl<T> OwnedWnfState<T>
where
    T: WnfWrite + ?Sized,
{
    pub fn set<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.raw.set(data)
    }

    pub fn update<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.raw.update(data, expected_change_stamp)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfWrite + ?Sized,
{
    pub fn set<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.raw.set(data)
    }

    pub fn update<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.raw.update(data, expected_change_stamp)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfWrite + ?Sized,
{
    pub fn set<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<T>,
    {
        Ok(data
            .borrow()
            .write(|buffer, buffer_size| self.update_raw(buffer, buffer_size, None))
            .ok()?)
    }

    pub fn update<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<T>,
    {
        let result = data
            .borrow()
            .write(|buffer, buffer_size| self.update_raw(buffer, buffer_size, Some(expected_change_stamp)));

        Ok(if result == STATUS_UNSUCCESSFUL {
            false
        } else {
            result.ok()?;
            true
        })
    }

    pub fn update_raw(
        &self,
        buffer: *const T,
        buffer_size: usize,
        expected_change_stamp: Option<WnfChangeStamp>,
    ) -> NTSTATUS {
        let matching_change_stamp = expected_change_stamp.unwrap_or_default().into();
        let check_stamp = expected_change_stamp.is_some() as u32;

        let result = unsafe {
            ntdll_sys::ZwUpdateWnfStateData(
                &self.state_name.opaque_value(),
                buffer.cast(),
                buffer_size as u32,
                ptr::null(),
                ptr::null(),
                matching_change_stamp,
                check_stamp,
            )
        };

        debug!(
            target: NTDLL_TARGET,
            ?result,
            input.state_name = %self.state_name,
            input.buffer_size = buffer_size,
            input.matching_change_stamp = matching_change_stamp,
            input.check_stamp = check_stamp,
            "ZwUpdateWnfStateData",
        );

        result
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfUpdateError {
    #[error("failed to update WNF state data: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

impl From<NTSTATUS> for WnfUpdateError {
    fn from(result: NTSTATUS) -> Self {
        let err: windows::core::Error = result.into();
        err.into()
    }
}
