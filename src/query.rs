use std::alloc::Layout;
use std::mem::MaybeUninit;
use std::{alloc, mem, ptr};

use thiserror::Error;
use tracing::debug;
use windows::Win32::Foundation::{NTSTATUS, STATUS_BUFFER_TOO_SMALL};

use crate::bytes::CheckedBitPattern;
use crate::data::{WnfChangeStamp, WnfStampedData};
use crate::ntdll::NTDLL_TARGET;
use crate::ntdll_sys;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};

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

    pub fn get_slice(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.raw.get_slice()
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        self.raw.query()
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError> {
        self.raw.query_boxed()
    }

    pub fn query_slice(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError> {
        self.raw.query_slice()
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

    pub fn get_slice(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.raw.get_slice()
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        self.raw.query()
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError> {
        self.raw.query_boxed()
    }

    pub fn query_slice(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError> {
        self.raw.query_slice()
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

    pub fn get_slice(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.query_slice().map(WnfStampedData::into_data)
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        let mut bits = MaybeUninit::<T::Bits>::uninit();

        let (size, change_stamp) = unsafe { self.query_internal(bits.as_mut_ptr(), mem::size_of::<T::Bits>()) }?;
        if size != mem::size_of::<T::Bits>() {
            return Err(WnfQueryError::WrongSize {
                expected: mem::size_of::<T::Bits>(),
                actual: size,
            });
        }

        let bits = unsafe { bits.assume_init() };

        if T::is_valid_bit_pattern(&bits) {
            let data = unsafe { *(&bits as *const T::Bits as *const T) };
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(WnfQueryError::InvalidBitPattern)
        }
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError> {
        let mut bits = if mem::size_of::<T::Bits>() == 0 {
            let data = unsafe { mem::zeroed() };
            Box::new(data)
        } else {
            let layout = Layout::new::<T::Bits>();
            let data = unsafe { alloc::alloc(layout) } as *mut MaybeUninit<T::Bits>;
            unsafe { Box::from_raw(data) }
        };

        let (size, change_stamp) = unsafe { self.query_internal(bits.as_mut_ptr(), mem::size_of::<T::Bits>()) }?;
        if size != mem::size_of::<T::Bits>() {
            return Err(WnfQueryError::WrongSize {
                expected: mem::size_of::<T::Bits>(),
                actual: size,
            });
        }

        let bits = unsafe { Box::from_raw(Box::into_raw(bits) as *mut T::Bits) };

        if T::is_valid_bit_pattern(&bits) {
            let data = unsafe { Box::from_raw(Box::into_raw(bits) as *mut T) };
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(WnfQueryError::InvalidBitPattern)
        }
    }

    pub fn query_slice(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError> {
        let stride = Layout::new::<T::Bits>().pad_to_align().size();
        let mut buffer: Vec<T::Bits> = Vec::new();

        let (len, change_stamp) = loop {
            let (size, change_stamp) = unsafe { self.query_internal(buffer.as_mut_ptr(), buffer.capacity() * stride) }?;

            if size == 0 {
                break (0, change_stamp);
            }

            if mem::size_of::<T::Bits>() == 0 {
                return Err(WnfQueryError::WrongSize {
                    expected: 0,
                    actual: size,
                });
            }

            if size % mem::size_of::<T::Bits>() != 0 {
                return Err(WnfQueryError::WrongSizeMultiple {
                    expected_modulus: mem::size_of::<T::Bits>(),
                    actual: size,
                });
            }

            let len = size / stride;
            if len > buffer.capacity() {
                buffer.reserve(len - buffer.capacity());
            } else {
                break (len, change_stamp);
            }
        };

        unsafe {
            buffer.set_len(len);
        }

        if buffer.iter().all(T::is_valid_bit_pattern) {
            let data = buffer.into_boxed_slice();
            let data = unsafe { Box::from_raw(Box::into_raw(data) as *mut [T]) };
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(WnfQueryError::InvalidBitPattern)
        }
    }

    unsafe fn query_internal(
        &self,
        buffer: *mut T::Bits,
        buffer_size: usize,
    ) -> Result<(usize, WnfChangeStamp), windows::core::Error> {
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
    #[error("failed to query WNF state data: data has wrong size (expected {expected}, got {actual})")]
    WrongSize { expected: usize, actual: usize },

    #[error(
        "failed to query WNF state data: data has wrong size (expected multiple of {expected_modulus}, got {actual})"
    )]
    WrongSizeMultiple { expected_modulus: usize, actual: usize },

    #[error("failed to query WNF state data: data has invalid bit pattern")]
    InvalidBitPattern,

    #[error("failed to query WNF state data: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

impl From<NTSTATUS> for WnfQueryError {
    fn from(result: NTSTATUS) -> Self {
        let err: windows::core::Error = result.into();
        err.into()
    }
}
