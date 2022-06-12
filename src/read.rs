use std::alloc::Layout;
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::{alloc, mem, ptr};

use thiserror::Error;

use crate::bytes::CheckedBitPattern;

pub trait WnfRead: Sized {
    unsafe fn from_buffer(ptr: *const c_void, size: usize) -> Option<Self>;

    unsafe fn from_reader<E, F, Meta>(reader: F) -> Result<(Self, Meta), E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut c_void, usize) -> Result<(usize, Meta), E>;
}

pub trait WnfReadBoxed {
    unsafe fn from_buffer_boxed(ptr: *const c_void, size: usize) -> Option<Box<Self>>;

    unsafe fn from_reader_boxed<E, F, Meta>(reader: F) -> Result<(Box<Self>, Meta), E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut c_void, usize) -> Result<(usize, Meta), E>;
}

impl<T> WnfRead for T
where
    T: CheckedBitPattern,
{
    unsafe fn from_buffer(ptr: *const c_void, size: usize) -> Option<Self> {
        if size != mem::size_of::<T::Bits>() {
            return None;
        }

        let bits: T::Bits = ptr::read_unaligned(ptr.cast());

        if T::is_valid_bit_pattern(&bits) {
            Some(*(&bits as *const T::Bits as *const T))
        } else {
            None
        }
    }

    unsafe fn from_reader<E, F, Meta>(mut reader: F) -> Result<(Self, Meta), E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut c_void, usize) -> Result<(usize, Meta), E>,
    {
        let mut bits = MaybeUninit::<T::Bits>::uninit();

        let (size, meta) = reader(bits.as_mut_ptr().cast(), mem::size_of::<T::Bits>())?;

        if size != mem::size_of::<T::Bits>() {
            return Err(E::from(WnfReadError::WrongSize {
                expected: mem::size_of::<T::Bits>(),
                actual: size,
            }));
        }

        let bits = bits.assume_init();

        if T::is_valid_bit_pattern(&bits) {
            let data = *(&bits as *const T::Bits as *const T);
            Ok((data, meta))
        } else {
            Err(E::from(WnfReadError::InvalidBitPattern))
        }
    }
}

impl<T> WnfReadBoxed for T
where
    T: CheckedBitPattern,
{
    unsafe fn from_buffer_boxed(ptr: *const c_void, size: usize) -> Option<Box<Self>> {
        if size != mem::size_of::<T::Bits>() {
            return None;
        }

        let bits = if mem::size_of::<T::Bits>() == 0 {
            Box::new(mem::zeroed())
        } else {
            let layout = Layout::new::<T::Bits>();
            let data = alloc::alloc(layout) as *mut T::Bits;
            ptr::copy_nonoverlapping(ptr as *const T::Bits, data, 1);
            Box::from_raw(data)
        };

        if T::is_valid_bit_pattern(&bits) {
            Some(Box::from_raw(Box::into_raw(bits) as *mut T))
        } else {
            None
        }
    }

    unsafe fn from_reader_boxed<E, F, Meta>(mut reader: F) -> Result<(Box<Self>, Meta), E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut c_void, usize) -> Result<(usize, Meta), E>,
    {
        let mut bits = if mem::size_of::<T::Bits>() == 0 {
            Box::new(mem::zeroed())
        } else {
            let layout = Layout::new::<T::Bits>();
            let data = alloc::alloc(layout) as *mut MaybeUninit<T::Bits>;
            Box::from_raw(data)
        };

        let (size, meta) = reader(bits.as_mut_ptr().cast(), mem::size_of::<T::Bits>())?;

        if size != mem::size_of::<T::Bits>() {
            return Err(E::from(WnfReadError::WrongSize {
                expected: mem::size_of::<T::Bits>(),
                actual: size,
            }));
        }

        let bits = Box::from_raw(Box::into_raw(bits) as *mut T::Bits);

        if T::is_valid_bit_pattern(&bits) {
            let data = Box::from_raw(Box::into_raw(bits) as *mut T);
            Ok((data, meta))
        } else {
            Err(E::from(WnfReadError::InvalidBitPattern))
        }
    }
}

impl<T> WnfReadBoxed for [T]
where
    T: CheckedBitPattern,
{
    unsafe fn from_buffer_boxed(ptr: *const c_void, size: usize) -> Option<Box<Self>> {
        if mem::size_of::<T>() == 0 {
            return (size == 0).then(|| Vec::new().into_boxed_slice());
        }

        if size % mem::size_of::<T>() != 0 {
            return None;
        }

        let len = size / mem::size_of::<T>();
        let mut buffer = Vec::with_capacity(len);
        ptr::copy_nonoverlapping(ptr.cast(), buffer.as_mut_ptr(), len);
        buffer.set_len(len);

        if buffer.iter().all(T::is_valid_bit_pattern) {
            let data = buffer.into_boxed_slice();
            Some(Box::from_raw(Box::into_raw(data) as *mut [T]))
        } else {
            None
        }
    }

    unsafe fn from_reader_boxed<E, F, Meta>(mut reader: F) -> Result<(Box<Self>, Meta), E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut c_void, usize) -> Result<(usize, Meta), E>,
    {
        let mut buffer: Vec<T::Bits> = Vec::new();

        let (len, meta) = loop {
            let (size, meta) = reader(
                buffer.as_mut_ptr().cast(),
                buffer.capacity() * mem::size_of::<T::Bits>(),
            )?;

            if size == 0 {
                break (0, meta);
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
                break (len, meta);
            }
        };

        buffer.set_len(len);

        if buffer.iter().all(T::is_valid_bit_pattern) {
            let data = buffer.into_boxed_slice();
            let data = Box::from_raw(Box::into_raw(data) as *mut [T]);
            Ok((data, meta))
        } else {
            Err(E::from(WnfReadError::InvalidBitPattern))
        }
    }
}

pub(crate) trait WnfReadRepr<T>
where
    T: ?Sized,
{
    type Data;

    unsafe fn from_buffer(ptr: *const c_void, size: usize) -> Option<Self::Data>;

    unsafe fn from_reader<E, F, Meta>(reader: F) -> Result<(Self::Data, Meta), E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut c_void, usize) -> Result<(usize, Meta), E>;
}

#[derive(Debug)]
pub(crate) enum Unboxed {}

impl<T> WnfReadRepr<T> for Unboxed
where
    T: WnfRead,
{
    type Data = T;

    unsafe fn from_buffer(ptr: *const c_void, size: usize) -> Option<T> {
        T::from_buffer(ptr, size)
    }

    unsafe fn from_reader<E, F, Meta>(reader: F) -> Result<(T, Meta), E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut c_void, usize) -> Result<(usize, Meta), E>,
    {
        T::from_reader(reader)
    }
}

#[derive(Debug)]
pub(crate) enum Boxed {}

impl<T> WnfReadRepr<T> for Boxed
where
    T: WnfReadBoxed + ?Sized,
{
    type Data = Box<T>;

    unsafe fn from_buffer(ptr: *const c_void, size: usize) -> Option<Box<T>> {
        T::from_buffer_boxed(ptr, size)
    }

    unsafe fn from_reader<E, F, Meta>(reader: F) -> Result<(Box<T>, Meta), E>
    where
        E: From<WnfReadError>,
        F: FnMut(*mut c_void, usize) -> Result<(usize, Meta), E>,
    {
        T::from_reader_boxed(reader)
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
