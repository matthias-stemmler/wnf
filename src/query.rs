use std::ptr;

use thiserror::Error;
use tracing::debug;
use windows::Win32::Foundation::{NTSTATUS, STATUS_BUFFER_TOO_SMALL};

use crate::data::{WnfChangeStamp, WnfStampedData};
use crate::ntdll::NTDLL_TARGET;
use crate::ntdll_sys;
use crate::read::{Boxed, Unboxed, WnfRead, WnfReadError, WnfReadRepr};
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};

impl<T> OwnedWnfState<T>
where
    T: WnfRead + ?Sized,
{
    pub fn get(&self) -> Result<T, WnfQueryError>
    where
        T: Sized,
    {
        self.raw.get()
    }

    pub fn get_boxed(&self) -> Result<Box<T>, WnfQueryError> {
        self.raw.get_boxed()
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError>
    where
        T: Sized,
    {
        self.raw.query()
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError> {
        self.raw.query_boxed()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead + ?Sized,
{
    pub fn get(&self) -> Result<T, WnfQueryError>
    where
        T: Sized,
    {
        self.raw.get()
    }

    pub fn get_boxed(&self) -> Result<Box<T>, WnfQueryError> {
        self.raw.get_boxed()
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError>
    where
        T: Sized,
    {
        self.raw.query()
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError> {
        self.raw.query_boxed()
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead + ?Sized,
{
    pub fn get(&self) -> Result<T, WnfQueryError>
    where
        T: Sized,
    {
        self.query().map(WnfStampedData::into_data)
    }

    pub fn get_boxed(&self) -> Result<Box<T>, WnfQueryError> {
        self.query_boxed().map(WnfStampedData::into_data)
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError>
    where
        T: Sized,
    {
        Ok(unsafe { Unboxed::<T>::read(|buffer, buffer_size| self.query_raw(buffer, buffer_size)) }?)
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError> {
        Ok(unsafe { Boxed::<T>::read(|buffer, buffer_size| self.query_raw(buffer, buffer_size)) }?)
    }

    unsafe fn query_raw(
        &self,
        buffer: *mut T::Bits,
        buffer_size: usize,
    ) -> Result<(usize, WnfChangeStamp), WnfQueryError> {
        let mut change_stamp = WnfChangeStamp::default();
        let mut size = buffer_size as u32;

        let result = ntdll_sys::ZwQueryWnfStateData(
            &self.state_name.opaque_value(),
            ptr::null(),
            ptr::null(),
            change_stamp.as_mut_ptr(),
            buffer.cast(),
            &mut size,
        );

        if result.is_err() && (result != STATUS_BUFFER_TOO_SMALL || size as usize <= buffer_size) {
            debug!(
                 target: NTDLL_TARGET,
                 ?result,
                 input.state_name = %self.state_name,
                 "ZwQueryWnfStateData",
            );

            Err(result.into())
        } else {
            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.state_name = %self.state_name,
                output.change_stamp = %change_stamp,
                output.buffer_size = size,
                "ZwQueryWnfStateData",
            );

            Ok((size as usize, change_stamp))
        }
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
