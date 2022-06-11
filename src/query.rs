use std::ffi::c_void;
use std::ptr;

use thiserror::Error;
use tracing::debug;
use windows::Win32::Foundation::{NTSTATUS, STATUS_BUFFER_TOO_SMALL};

use crate::bytes::CheckedBitPattern;
use crate::data::{WnfChangeStamp, WnfStampedData};
use crate::ntdll::NTDLL_TARGET;
use crate::read::{Boxed, Unboxed, WnfRead, WnfReadError};
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::{ntdll_sys, WnfStateName};

impl<T> OwnedWnfState<T>
where
    T: CheckedBitPattern,
{
    pub fn get(&self) -> Result<T, WnfQueryError> {
        self.raw.get()
    }

    pub fn get_boxed(&self) -> Result<Box<T>, WnfQueryError> {
        self.raw.get_boxed()
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        self.raw.query()
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError> {
        self.raw.query_boxed()
    }
}

impl<T> OwnedWnfState<[T]>
where
    T: CheckedBitPattern,
{
    pub fn get_boxed(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.raw.get_boxed()
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError> {
        self.raw.query_boxed()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: CheckedBitPattern,
{
    pub fn get(&self) -> Result<T, WnfQueryError> {
        self.raw.get()
    }

    pub fn get_boxed(&self) -> Result<Box<T>, WnfQueryError> {
        self.raw.get_boxed()
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        self.raw.query()
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError> {
        self.raw.query_boxed()
    }
}

impl<T> BorrowedWnfState<'_, [T]>
where
    T: CheckedBitPattern,
{
    pub fn get_boxed(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.raw.get_boxed()
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError> {
        self.raw.query_boxed()
    }
}

impl<T> RawWnfState<T>
where
    T: CheckedBitPattern,
{
    pub fn get(&self) -> Result<T, WnfQueryError> {
        self.query().map(WnfStampedData::into_data)
    }

    pub fn get_boxed(&self) -> Result<Box<T>, WnfQueryError> {
        self.query_boxed().map(WnfStampedData::into_data)
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        let data = unsafe { Unboxed::from_reader(|buffer, buffer_size| query(self.state_name, buffer, buffer_size)) }?;
        Ok(data.into())
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError> {
        let data = unsafe {
            <Boxed as WnfRead<T>>::from_reader(|buffer, buffer_size| query(self.state_name, buffer, buffer_size))
        }?;
        Ok(data.into())
    }
}

impl<T> RawWnfState<[T]>
where
    T: CheckedBitPattern,
{
    pub fn get_boxed(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.query_boxed().map(WnfStampedData::into_data)
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError> {
        let data = unsafe {
            <Boxed as WnfRead<[T]>>::from_reader(|buffer, buffer_size| query(self.state_name, buffer, buffer_size))
        }?;
        Ok(data.into())
    }
}

unsafe fn query(
    state_name: WnfStateName,
    buffer: *mut c_void,
    buffer_size: usize,
) -> Result<(usize, WnfChangeStamp), WnfQueryError> {
    let mut change_stamp = WnfChangeStamp::default();
    let mut size = buffer_size as u32;

    let result = ntdll_sys::ZwQueryWnfStateData(
        &state_name.opaque_value(),
        ptr::null(),
        ptr::null(),
        change_stamp.as_mut_ptr(),
        buffer,
        &mut size,
    );

    if result.is_err() && (result != STATUS_BUFFER_TOO_SMALL || size as usize <= buffer_size) {
        debug!(
             target: NTDLL_TARGET,
             ?result,
             input.state_name = %state_name,
             "ZwQueryWnfStateData",
        );

        Err(result.into())
    } else {
        debug!(
            target: NTDLL_TARGET,
            ?result,
            input.state_name = %state_name,
            output.change_stamp = %change_stamp,
            output.buffer_size = size,
            "ZwQueryWnfStateData",
        );

        Ok((size as usize, change_stamp))
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfQueryError {
    #[error("failed to query WNF state data: {0}")]
    Read(#[from] WnfReadError),

    #[error("failed to query WNF state data: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

impl From<NTSTATUS> for WnfQueryError {
    fn from(result: NTSTATUS) -> Self {
        let err: windows::core::Error = result.into();
        err.into()
    }
}
