use crate::buffer::FromByteSourceError::Invalid;
use crate::bytes::CheckedBitPattern;
use std::alloc::{self, Layout};
use std::convert::Infallible;
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::{mem, ptr};

// TODO Split in with/without known size
unsafe trait ByteSource {
    type Error;

    fn known_size(&self) -> Option<usize>;
    unsafe fn read(&mut self, ptr: *mut c_void, size: usize) -> Result<usize, Self::Error>;
}

struct FnByteSource<F>(F);

impl<F, E> FnByteSource<F>
where
    F: FnMut(*mut c_void, usize) -> Result<usize, E>,
{
    unsafe fn new(f: F) -> Self {
        Self(f)
    }
}

unsafe impl<F, E> ByteSource for FnByteSource<F>
where
    F: FnMut(*mut c_void, usize) -> Result<usize, E>,
{
    type Error = E;

    fn known_size(&self) -> Option<usize> {
        None
    }

    unsafe fn read(&mut self, ptr: *mut c_void, size: usize) -> Result<usize, Self::Error> {
        (self.0)(ptr, size)
    }
}

struct BufferByteSource {
    ptr: *mut c_void,
    size: usize,
}

impl BufferByteSource {
    unsafe fn new(ptr: *mut c_void, size: usize) -> Self {
        Self { ptr, size }
    }
}

unsafe impl ByteSource for BufferByteSource {
    type Error = Infallible;

    fn known_size(&self) -> Option<usize> {
        Some(self.size)
    }

    unsafe fn read(&mut self, ptr: *mut c_void, size: usize) -> Result<usize, Self::Error> {
        debug_assert!(size == self.size);
        ptr::copy_nonoverlapping(self.ptr, ptr, size);
        Ok(size)
    }
}

enum Boxed {}

trait FromByteSource<A = ()>: Sized {
    fn from_byte_source<S>(source: &mut S) -> Result<Self, FromByteSourceError<S::Error>>
    where
        S: ByteSource;
}

enum FromByteSourceError<E> {
    Read(E),
    WrongSize,
    Invalid,
}

impl<E> From<E> for FromByteSourceError<E> {
    fn from(err: E) -> Self {
        Self::Read(err)
    }
}

impl<T> FromByteSource for T
where
    T: CheckedBitPattern,
{
    fn from_byte_source<S>(source: &mut S) -> Result<Self, FromByteSourceError<S::Error>>
    where
        S: ByteSource,
    {
        let mut data = MaybeUninit::<T::Bits>::uninit();

        unsafe { read(source, data.as_mut_ptr()) }?;

        let bits = unsafe { data.assume_init() };

        if T::is_valid_bit_pattern(&bits) {
            Ok(unsafe { *(&bits as *const T::Bits as *const T) })
        } else {
            Err(FromByteSourceError::Invalid)
        }
    }
}

impl<T> FromByteSource<Boxed> for Box<T>
where
    T: CheckedBitPattern,
{
    fn from_byte_source<S>(source: &mut S) -> Result<Self, FromByteSourceError<S::Error>>
    where
        S: ByteSource,
    {
        let mut data: Box<MaybeUninit<T::Bits>> = if mem::size_of::<T>() == 0 {
            Box::new(MaybeUninit::uninit())
        } else {
            let layout = Layout::new::<T::Bits>();
            let data = unsafe { alloc::alloc(layout) };
            unsafe { Box::from_raw(data as *mut MaybeUninit<T::Bits>) }
        };

        unsafe { read(source, data.as_mut_ptr()) }?;

        let bits = unsafe { Box::from_raw(Box::into_raw(data) as *mut T::Bits) };

        if T::is_valid_bit_pattern(&bits) {
            Ok(unsafe { Box::from_raw(Box::into_raw(bits) as *mut T) })
        } else {
            Err(FromByteSourceError::Invalid)
        }
    }
}

unsafe fn read<S, T>(source: &mut S, buffer: *mut T) -> Result<(), FromByteSourceError<S::Error>>
where
    S: ByteSource,
{
    match source.known_size() {
        Some(size) if size == mem::size_of::<T>() => {
            source.read(buffer as *mut c_void, mem::size_of::<T>())?;
        }
        Some(..) => return Err(FromByteSourceError::WrongSize),
        None => {
            let size = source.read(buffer as *mut c_void, mem::size_of::<T>())?;
            if size != mem::size_of::<T>() {
                return Err(FromByteSourceError::WrongSize);
            }
        }
    }

    Ok(())
}

impl<T> FromByteSource for Box<[T]>
where
    T: CheckedBitPattern,
{
    fn from_byte_source<S>(source: &mut S) -> Result<Self, FromByteSourceError<S::Error>>
    where
        S: ByteSource,
    {
        let data: Vec<T::Bits> = if mem::size_of::<T::Bits>() == 0 {
            let size = source
                .known_size()
                .map(Ok)
                .unwrap_or_else(|| unsafe { source.read(ptr::null_mut(), 0) })?;

            if size == 0 {
                Vec::new()
            } else {
                return Err(FromByteSourceError::WrongSize);
            }
        } else {
            match source.known_size() {
                Some(size) if size % mem::size_of::<T::Bits>() == 0 => {
                    let len = size / mem::size_of::<T::Bits>();
                    let mut data = Vec::with_capacity(len);
                    unsafe { source.read(data.as_mut_ptr() as *mut c_void, size) }?;
                    unsafe { data.set_len(len) };
                    data
                }
                Some(..) => return Err(FromByteSourceError::WrongSize),
                None => {
                    let mut data = Vec::new();

                    let len = loop {
                        let size = unsafe {
                            source.read(
                                data.as_mut_ptr() as *mut c_void,
                                data.capacity() * mem::size_of::<T::Bits>(),
                            )
                        }?;
                        if size % mem::size_of::<T::Bits>() != 0 {
                            return Err(FromByteSourceError::WrongSize);
                        }

                        let len = size / mem::size_of::<T::Bits>();
                        if len <= data.capacity() {
                            break len;
                        } else {
                            data.reserve(len - data.capacity());
                        }
                    };

                    unsafe { data.set_len(len) };
                    data
                }
            }
        };

        if data.iter().all(T::is_valid_bit_pattern) {
            let data = data.into_boxed_slice();
            let data = unsafe { Box::from_raw(Box::into_raw(data) as *mut [T]) };
            Ok(data)
        } else {
            Err(Invalid)
        }
    }
}
