use std::alloc::Layout;
use std::ffi::c_void;
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
    type Bits = T::Bits;

    unsafe fn read<E, F>(mut read_raw: F) -> Result<WnfStampedData<T>, E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut T::Bits, usize) -> Result<(usize, WnfChangeStamp), E>,
    {
        let mut bits = MaybeUninit::<T::Bits>::uninit();

        let (size, change_stamp) = read_raw(bits.as_mut_ptr(), mem::size_of::<T::Bits>())?;
        if size != mem::size_of::<T::Bits>() {
            return Err(E::from(WnfReadError::WrongSize {
                expected: mem::size_of::<T::Bits>(),
                actual: size,
            }));
        }

        let bits = bits.assume_init();

        if T::is_valid_bit_pattern(&bits) {
            let data = *(&bits as *const T::Bits as *const Self);
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(E::from(WnfReadError::InvalidBitPattern))
        }
    }

    unsafe fn read_boxed<E, F>(mut read_raw: F) -> Result<WnfStampedData<Box<T>>, E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut T::Bits, usize) -> Result<(usize, WnfChangeStamp), E>,
    {
        let mut bits = if mem::size_of::<T::Bits>() == 0 {
            let data = mem::zeroed();
            Box::new(data)
        } else {
            let layout = Layout::new::<T::Bits>();
            let data = alloc::alloc(layout) as *mut MaybeUninit<T::Bits>;
            Box::from_raw(data)
        };

        let (size, change_stamp) = read_raw(bits.as_mut_ptr(), mem::size_of::<T::Bits>())?;
        if size != mem::size_of::<T::Bits>() {
            return Err(E::from(WnfReadError::WrongSize {
                expected: mem::size_of::<T::Bits>(),
                actual: size,
            }));
        }

        let bits = Box::from_raw(Box::into_raw(bits) as *mut T::Bits);

        if T::is_valid_bit_pattern(&bits) {
            let data = Box::from_raw(Box::into_raw(bits) as *mut T);
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(E::from(WnfReadError::InvalidBitPattern))
        }
    }

    unsafe fn read_buffer(ptr: *const c_void, size: u32) -> Option<T> {
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

    unsafe fn read_buffer_boxed(ptr: *const c_void, size: u32) -> Option<Box<T>> {
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

    unsafe fn read<E, F>(_: F) -> Result<WnfStampedData<[T]>, E>
    where
        [T]: Sized,
    {
        unreachable!("slice is unsized")
    }

    unsafe fn read_boxed<E, F>(mut read_raw: F) -> Result<WnfStampedData<Box<[T]>>, E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut T::Bits, usize) -> Result<(usize, WnfChangeStamp), E>,
    {
        let mut buffer: Vec<T::Bits> = Vec::new();

        let (len, change_stamp) = loop {
            let (size, change_stamp) = read_raw(buffer.as_mut_ptr(), buffer.capacity() * mem::size_of::<T::Bits>())?;

            if size == 0 {
                break (0, change_stamp);
            }

            if mem::size_of::<T::Bits>() == 0 {
                return Err(E::from(WnfReadError::WrongSize {
                    expected: 0,
                    actual: size,
                }));
            }

            if size % mem::size_of::<T::Bits>() != 0 {
                return Err(E::from(WnfReadError::WrongSizeMultiple {
                    expected_modulus: mem::size_of::<T::Bits>(),
                    actual: size,
                }));
            }

            let len = size / mem::size_of::<T::Bits>();
            if len > buffer.capacity() {
                buffer.reserve(len - buffer.capacity());
            } else {
                break (len, change_stamp);
            }
        };

        buffer.set_len(len);

        if buffer.iter().all(T::is_valid_bit_pattern) {
            let data = buffer.into_boxed_slice();
            let data = Box::from_raw(Box::into_raw(data) as *mut [T]);
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(E::from(WnfReadError::InvalidBitPattern))
        }
    }

    unsafe fn read_buffer(_: *const c_void, _: u32) -> Option<[T]>
    where
        [T]: Sized,
    {
        unreachable!("slice is unsized")
    }

    unsafe fn read_buffer_boxed(ptr: *const c_void, size: u32) -> Option<Box<[T]>> {
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

pub(crate) trait WnfReadRepr<T>
where
    T: WnfRead + ?Sized,
{
    type Data;

    unsafe fn read<E, F>(read_raw: F) -> Result<WnfStampedData<Self::Data>, E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut T::Bits, usize) -> Result<(usize, WnfChangeStamp), E>;

    unsafe fn read_buffer(ptr: *const c_void, size: u32) -> Option<Self::Data>;
}

#[derive(Debug)]
pub(crate) struct Unboxed;

impl<T> WnfReadRepr<T> for Unboxed
where
    T: WnfRead,
{
    type Data = T;

    unsafe fn read<E, F>(read_raw: F) -> Result<WnfStampedData<T>, E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut T::Bits, usize) -> Result<(usize, WnfChangeStamp), E>,
    {
        T::read(read_raw)
    }

    unsafe fn read_buffer(ptr: *const c_void, size: u32) -> Option<T> {
        T::read_buffer(ptr, size)
    }
}

#[derive(Debug)]
pub(crate) struct Boxed;

impl<T> WnfReadRepr<T> for Boxed
where
    T: WnfRead + ?Sized,
{
    type Data = Box<T>;

    unsafe fn read<E, F>(read_raw: F) -> Result<WnfStampedData<Box<T>>, E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut T::Bits, usize) -> Result<(usize, WnfChangeStamp), E>,
    {
        T::read_boxed(read_raw)
    }

    unsafe fn read_buffer(ptr: *const c_void, size: u32) -> Option<Box<T>> {
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
