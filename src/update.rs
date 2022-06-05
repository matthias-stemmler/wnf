use std::borrow::Borrow;
use std::{mem, ptr, slice};

use thiserror::Error;
use tracing::debug;
use windows::Win32::Foundation::{NTSTATUS, STATUS_UNSUCCESSFUL};

use crate::bytes::NoUninit;
use crate::data::WnfChangeStamp;
use crate::ntdll::NTDLL_TARGET;
use crate::ntdll_sys;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};

impl<T> OwnedWnfState<T>
where
    T: NoUninit,
{
    pub fn set<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.raw.set(data)
    }

    pub fn set_slice<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<[T]>,
    {
        self.raw.set_slice(data)
    }

    pub fn update<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.raw.update(data, expected_change_stamp)
    }

    pub fn update_slice<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<[T]>,
    {
        self.raw.update_slice(data, expected_change_stamp)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: NoUninit,
{
    pub fn set<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.raw.set(data)
    }

    pub fn set_slice<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<[T]>,
    {
        self.raw.set_slice(data)
    }

    pub fn update<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.raw.update(data, expected_change_stamp)
    }

    pub fn update_slice<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<[T]>,
    {
        self.raw.update_slice(data, expected_change_stamp)
    }
}

impl<T> RawWnfState<T>
where
    T: NoUninit,
{
    pub fn set<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.set_slice(slice::from_ref(data.borrow()))
    }

    pub fn set_slice<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<[T]>,
    {
        Ok(self.update_slice_internal(data, None).ok()?)
    }

    pub fn update<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.update_slice(slice::from_ref(data.borrow()), expected_change_stamp)
    }

    pub fn update_slice<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<[T]>,
    {
        let result = self.update_slice_internal(data, Some(expected_change_stamp));

        Ok(if result == STATUS_UNSUCCESSFUL {
            false
        } else {
            result.ok()?;
            true
        })
    }

    pub fn update_slice_internal<D>(&self, data: D, expected_change_stamp: Option<WnfChangeStamp>) -> NTSTATUS
    where
        D: Borrow<[T]>,
    {
        let data = data.borrow();
        let buffer_size = (data.len() * mem::size_of::<T>()) as u32; // T: NoUninit should imply that this is the correct size
        let matching_change_stamp = expected_change_stamp.unwrap_or_default().into();
        let check_stamp = expected_change_stamp.is_some() as u32;

        let result = unsafe {
            ntdll_sys::ZwUpdateWnfStateData(
                &self.state_name.opaque_value(),
                data.as_ptr().cast(),
                buffer_size,
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
