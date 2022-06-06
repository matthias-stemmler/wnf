use std::alloc::Layout;
use std::mem::MaybeUninit;
use std::{alloc, mem};

use thiserror::Error;

use crate::bytes::{AnyBitPattern, CheckedBitPattern};
use crate::{WnfChangeStamp, WnfStampedData};

pub trait WnfRead {
    type Bits: AnyBitPattern;

    unsafe fn read<E, F>(read_raw: F) -> Result<WnfStampedData<Self>, E>
    where
        Self: Sized,
        E: From<WnfReadError>,
        F: FnMut(*mut Self::Bits, usize) -> Result<(usize, WnfChangeStamp), E>;

    unsafe fn read_boxed<E, F>(read_raw: F) -> Result<WnfStampedData<Box<Self>>, E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut Self::Bits, usize) -> Result<(usize, WnfChangeStamp), E>;
}

impl<T> WnfRead for T
where
    T: CheckedBitPattern,
{
    type Bits = <Self as CheckedBitPattern>::Bits;

    unsafe fn read<E, F>(mut read_raw: F) -> Result<WnfStampedData<Self>, E>
    where
        Self: Sized,
        E: From<WnfReadError>,
        F: FnMut(*mut Self::Bits, usize) -> Result<(usize, WnfChangeStamp), E>,
    {
        let mut bits = MaybeUninit::<Self::Bits>::uninit();

        let (size, change_stamp) = read_raw(bits.as_mut_ptr(), mem::size_of::<Self::Bits>())?;
        if size != mem::size_of::<Self::Bits>() {
            return Err(E::from(WnfReadError::WrongSize {
                expected: mem::size_of::<Self::Bits>(),
                actual: size,
            }));
        }

        let bits = bits.assume_init();

        if Self::is_valid_bit_pattern(&bits) {
            let data = *(&bits as *const Self::Bits as *const Self);
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(E::from(WnfReadError::InvalidBitPattern))
        }
    }

    unsafe fn read_boxed<E, F>(mut read_raw: F) -> Result<WnfStampedData<Box<Self>>, E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut Self::Bits, usize) -> Result<(usize, WnfChangeStamp), E>,
    {
        let mut bits = if mem::size_of::<Self::Bits>() == 0 {
            let data = mem::zeroed();
            Box::new(data)
        } else {
            let layout = Layout::new::<Self::Bits>();
            let data = alloc::alloc(layout) as *mut MaybeUninit<Self::Bits>;
            Box::from_raw(data)
        };

        let (size, change_stamp) = read_raw(bits.as_mut_ptr(), mem::size_of::<Self::Bits>())?;
        if size != mem::size_of::<Self::Bits>() {
            return Err(E::from(WnfReadError::WrongSize {
                expected: mem::size_of::<Self::Bits>(),
                actual: size,
            }));
        }

        let bits = Box::from_raw(Box::into_raw(bits) as *mut Self::Bits);

        if Self::is_valid_bit_pattern(&bits) {
            let data = Box::from_raw(Box::into_raw(bits) as *mut Self);
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(E::from(WnfReadError::InvalidBitPattern))
        }
    }
}

impl<T> WnfRead for [T]
where
    T: CheckedBitPattern,
{
    type Bits = T::Bits;

    unsafe fn read<E, F>(_: F) -> Result<WnfStampedData<Self>, E>
    where
        Self: Sized,
        E: From<WnfReadError>,
        F: FnMut(*mut Self::Bits, usize) -> Result<(usize, WnfChangeStamp), E>,
    {
        unreachable!("slice is unsized")
    }

    unsafe fn read_boxed<E, F>(mut read_raw: F) -> Result<WnfStampedData<Box<Self>>, E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut Self::Bits, usize) -> Result<(usize, WnfChangeStamp), E>,
    {
        let stride = Layout::new::<Self::Bits>().pad_to_align().size();
        let mut buffer: Vec<Self::Bits> = Vec::new();

        let (len, change_stamp) = loop {
            let (size, change_stamp) = read_raw(buffer.as_mut_ptr(), buffer.capacity() * stride)?;

            if size == 0 {
                break (0, change_stamp);
            }

            if mem::size_of::<Self::Bits>() == 0 {
                return Err(E::from(WnfReadError::WrongSize {
                    expected: 0,
                    actual: size,
                }));
            }

            if size % mem::size_of::<Self::Bits>() != 0 {
                return Err(E::from(WnfReadError::WrongSizeMultiple {
                    expected_modulus: mem::size_of::<Self::Bits>(),
                    actual: size,
                }));
            }

            let len = size / stride;
            if len > buffer.capacity() {
                buffer.reserve(len - buffer.capacity());
            } else {
                break (len, change_stamp);
            }
        };

        buffer.set_len(len);

        if buffer.iter().all(T::is_valid_bit_pattern) {
            let data = buffer.into_boxed_slice();
            let data = Box::from_raw(Box::into_raw(data) as *mut Self);
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(E::from(WnfReadError::InvalidBitPattern))
        }
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfReadError {
    #[error("failed to read WNF state data: data has wrong size (expected {expected}, got {actual})")]
    WrongSize { expected: usize, actual: usize },

    #[error(
        "failed to read WNF state data: data has wrong size (expected multiple of {expected_modulus}, got {actual})"
    )]
    WrongSizeMultiple { expected_modulus: usize, actual: usize },

    #[error("failed to read WNF state data: data has invalid bit pattern")]
    InvalidBitPattern,
}
