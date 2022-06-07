use std::alloc::Layout;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::{alloc, mem, ptr};

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

    unsafe fn read_buffer(ptr: *const c_void, size: u32) -> Option<Self>
    where
        Self: Sized;

    unsafe fn read_buffer_boxed(ptr: *const c_void, size: u32) -> Option<Box<Self>>;
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

    unsafe fn read_buffer(ptr: *const c_void, size: u32) -> Option<Self> {
        if size as usize != mem::size_of::<T::Bits>() {
            return None;
        }

        let bits: T::Bits = ptr::read_unaligned(ptr.cast());

        if T::is_valid_bit_pattern(&bits) {
            Some(*(&bits as *const T::Bits as *const T))
        } else {
            None
        }
    }

    unsafe fn read_buffer_boxed(ptr: *const c_void, size: u32) -> Option<Box<Self>> {
        if size as usize != mem::size_of::<T::Bits>() {
            return None;
        }

        let bits = if mem::size_of::<T::Bits>() == 0 {
            Box::new(mem::zeroed())
        } else {
            let layout = Layout::new::<T::Bits>();
            let data = alloc::alloc(layout) as *mut T::Bits;
            ptr::copy_nonoverlapping(ptr as *const T::Bits, data as *mut T::Bits, 1);
            Box::from_raw(data)
        };

        T::is_valid_bit_pattern(&bits).then(|| Box::from_raw(Box::into_raw(bits) as *mut T))
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

    unsafe fn read_buffer(_: *const c_void, _: u32) -> Option<Self>
    where
        Self: Sized,
    {
        unreachable!("slice is unsized")
    }

    unsafe fn read_buffer_boxed(ptr: *const c_void, size: u32) -> Option<Box<Self>> {
        if mem::size_of::<T>() == 0 {
            return (size == 0).then(|| Vec::new().into_boxed_slice());
        }

        if size as usize % mem::size_of::<T>() != 0 {
            return None;
        }

        let len = size as usize / mem::size_of::<T>();
        let mut data = Vec::with_capacity(len);
        ptr::copy_nonoverlapping(ptr.cast(), data.as_mut_ptr(), len);
        data.set_len(len);

        if data.iter().all(T::is_valid_bit_pattern) {
            let mut data = ManuallyDrop::new(data);
            let data = Vec::from_raw_parts(data.as_mut_ptr() as *mut T, data.len(), data.capacity());
            Some(data.into_boxed_slice())
        } else {
            None
        }
    }
}

pub(crate) trait WnfReadRepr {
    type Bits: AnyBitPattern;
    type Data;

    unsafe fn read<E, F>(read_raw: F) -> Result<WnfStampedData<Self::Data>, E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut Self::Bits, usize) -> Result<(usize, WnfChangeStamp), E>;

    unsafe fn read_buffer(ptr: *const c_void, size: u32) -> Option<Self::Data>;
}

#[derive(Debug)]
pub(crate) struct Unboxed<T>(PhantomData<fn() -> T>);

impl<T> WnfReadRepr for Unboxed<T>
where
    T: WnfRead,
{
    type Bits = T::Bits;
    type Data = T;

    unsafe fn read<E, F>(read_raw: F) -> Result<WnfStampedData<Self::Data>, E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut Self::Bits, usize) -> Result<(usize, WnfChangeStamp), E>,
    {
        T::read(read_raw)
    }

    unsafe fn read_buffer(ptr: *const c_void, size: u32) -> Option<Self::Data> {
        T::read_buffer(ptr, size)
    }
}

#[derive(Debug)]
pub(crate) struct Boxed<T>(PhantomData<fn() -> Box<T>>)
where
    T: ?Sized;

impl<T> WnfReadRepr for Boxed<T>
where
    T: WnfRead + ?Sized,
{
    type Bits = T::Bits;
    type Data = Box<T>;

    unsafe fn read<E, F>(read_raw: F) -> Result<WnfStampedData<Self::Data>, E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut Self::Bits, usize) -> Result<(usize, WnfChangeStamp), E>,
    {
        T::read_boxed(read_raw)
    }

    unsafe fn read_buffer(ptr: *const c_void, size: u32) -> Option<Self::Data> {
        T::read_buffer_boxed(ptr, size)
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
