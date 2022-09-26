use std::alloc::Layout;
use std::ffi::c_void;
use std::io::ErrorKind;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::{alloc, io, mem, ptr};

use thiserror::Error;

use crate::bytes::CheckedBitPattern;
use crate::data::WnfOpaqueData;

/// Trait for types that can be read from WNF state data
///
/// A type `T` implements [`WnfRead<D>`] if the data of a WNF state of type `T` (i.e., an [`OwnedWnfState<T>`] or a
/// [`BorrowedWnfState<'_, T>`]) can be read as an instance of type `D`, where `D` is either `T` or [`Box<T>`].
///
/// This is used to abstract over types that either implement [`CheckedBitPattern`] themselves or are of the form `[T]`
/// where `T` implements [`CheckedBitPattern`].
///
/// This trait is sealed and cannot be implemented outside of `wnf`.
pub trait WnfRead<D>: private::Sealed + Send + 'static {
    /// Tries to read a `D` from a preallocated buffer
    ///
    /// The buffer starts at `ptr` and is `size` bytes long.
    ///
    /// # Safety
    /// - `ptr` must be valid for reads of size `size`
    /// - The memory range of size `read_size` starting at `ptr` must be initialized
    unsafe fn from_buffer(ptr: *const c_void, size: usize) -> io::Result<D>;

    /// Tries to read a `D` by invoking a reader closure
    ///
    /// The reader closure takes a pointer to a buffer and the size of the buffer in bytes and tries to read a `D` into
    /// that buffer. It returns the actual number of bytes read and some metadata (such as a change stamp) that is
    /// passed through.
    ///
    /// # Safety
    /// When `reader` is invoked as `reader(ptr, size)`, it can assume that `ptr` is valid for writes of size `size`
    ///
    /// When `reader(ptr, size)` returns `Ok((read_size, _))`, it must guarantee that
    /// - `read_size <= size`
    /// - `ptr` is valid for reads of size `read_size`
    /// - The memory range of size `read_size` starting at `ptr` is initialized
    unsafe fn from_reader<F, Meta>(reader: F) -> io::Result<(D, Meta)>
    where
        F: FnMut(*mut c_void, usize) -> io::Result<(usize, Meta)>;
}

impl WnfRead<WnfOpaqueData> for WnfOpaqueData {
    unsafe fn from_buffer(_: *const c_void, _: usize) -> io::Result<WnfOpaqueData> {
        // We just produce a `WnfOpaqueData`, ignoring the buffer
        Ok(WnfOpaqueData::new())
    }

    unsafe fn from_reader<F, Meta>(mut reader: F) -> io::Result<(WnfOpaqueData, Meta)>
    where
        F: FnMut(*mut c_void, usize) -> io::Result<(usize, Meta)>,
    {
        // We have to invoke the reader in order to obtain the metadata
        // The precondition of `reader` is satisfied because `NonNull::dangling()` is valid for zero-size writes
        let (_, meta) = reader(NonNull::dangling().as_ptr(), 0)?;
        Ok((WnfOpaqueData::new(), meta))
    }
}

impl<T> WnfRead<T> for T
where
    T: CheckedBitPattern,
{
    unsafe fn from_buffer(ptr: *const c_void, size: usize) -> io::Result<T> {
        if size != mem::size_of::<T::Bits>() {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                WnfReadError::WrongSize {
                    expected: mem::size_of::<T::Bits>(),
                    actual: size,
                },
            ));
        }

        // SAFETY:
        // - `ptr` is valid for reads of `T::Bits` by the safety condition and `size == mem::size_of::<T::Bits>()`
        // - `ptr` points to a valid `T::Bits` because the memory range is initialized (by the safety condition)
        //    and `T::Bits: AnyBitPattern`
        let bits: T::Bits = ptr::read_unaligned(ptr.cast());

        if T::is_valid_bit_pattern(&bits) {
            // SAFETY: By the safety conditions of `CheckedBitPattern`,
            // - `T` has the same memory layout as `T::Bits`
            // - `bits` can be reinterpreted as a `T` because `T::is_valid_bit_pattern(&bits)` is `true`
            Ok(*(&bits as *const T::Bits as *const T))
        } else {
            Err(io::Error::new(ErrorKind::InvalidData, WnfReadError::InvalidBitPattern))
        }
    }

    unsafe fn from_reader<F, Meta>(mut reader: F) -> io::Result<(T, Meta)>
    where
        F: FnMut(*mut c_void, usize) -> io::Result<(usize, Meta)>,
    {
        let mut bits = MaybeUninit::<T::Bits>::uninit();

        // The precondition of `reader` is satisfied because `bits.as_mut_ptr()` is valid for writes of `T::Bits`
        let (size, meta) = reader(bits.as_mut_ptr().cast(), mem::size_of::<T::Bits>())?;

        if size != mem::size_of::<T::Bits>() {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                WnfReadError::WrongSize {
                    expected: mem::size_of::<T::Bits>(),
                    actual: size,
                },
            ));
        }

        // SAFETY: `bits.as_mut_ptr()`
        // - is valid for reads of `T::Bits` by the safety condition and `size == mem::size_of::<T::Bits>()`
        // - points to a valid `T::Bits` because the memory range is initialized (by the safety condition)
        //   and `T::Bits: AnyBitPattern`
        let bits = bits.assume_init();

        if T::is_valid_bit_pattern(&bits) {
            // SAFETY: By the safety conditions of `CheckedBitPattern`,
            // - `T` has the same memory layout as `T::Bits`
            // - `bits` can be reinterpreted as a `T` because `T::is_valid_bit_pattern(&bits)` is `true`
            let data = *(&bits as *const T::Bits as *const T);
            Ok((data, meta))
        } else {
            Err(io::Error::new(ErrorKind::InvalidData, WnfReadError::InvalidBitPattern))
        }
    }
}

impl<T> WnfRead<Box<T>> for T
where
    T: CheckedBitPattern,
{
    unsafe fn from_buffer(ptr: *const c_void, size: usize) -> io::Result<Box<T>> {
        if size != mem::size_of::<T::Bits>() {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                WnfReadError::WrongSize {
                    expected: mem::size_of::<T::Bits>(),
                    actual: size,
                },
            ));
        }

        // Ideally, we would use `Box::new_uninit`, but that is unstable
        let bits = if mem::size_of::<T::Bits>() == 0 {
            // SAFETY:
            // The all-zero byte pattern is a valid `T::Bits` because `T::Bits` is zero-sized
            // (or, alternatively, because `T::Bits: AnyBitPattern`)
            Box::new(mem::zeroed())
        } else {
            let layout = Layout::new::<T::Bits>();

            // SAFETY:
            // `layout` has non-zero size
            let data = alloc::alloc(layout) as *mut T::Bits;

            // SAFETY:
            // - `ptr` is valid for reads of `mem::size_of::<T::Bits>()` by the safety condition and
            //   `size == mem::size_of::<T::Bits>()`
            // - `data` is valid for writes of `mem::size_of::<T::Bits>()` because it was allocated with that size
            // - Both `ptr` and `data` are trivially properly aligned as `mem::size_of::<u8>() == 1`
            // - The source and destination regions don't overlap because the source region is within the bounds of a
            //   single allocated object (because `ptr` is valid for reads) while the destination region is a freshly
            //   allocated object
            ptr::copy_nonoverlapping(ptr as *const u8, data as *mut u8, mem::size_of::<T::Bits>());

            // SAFETY:
            // - `data` was allocated with the global allocator using the layout of `T::Bits`
            // - `data` is not aliased
            // - `data` points to a valid `T::Bits` because the memory range is initialized (by the safety condition)
            //   and `T::Bits: AnyBitPattern`
            Box::from_raw(data)
        };

        if T::is_valid_bit_pattern(&bits) {
            // SAFETY:
            // - The raw pointer is obtained via `Box::into_raw` from a `Box<T::Bits>`
            //
            // By the safety conditions of `CheckedBitPattern`,
            // - `T` has the same memory layout as `T::Bits`
            // - `bits` can be reinterpreted as a `T` because `T::is_valid_bit_pattern(&bits)` is `true`
            Ok(Box::from_raw(Box::into_raw(bits) as *mut T))
        } else {
            Err(io::Error::new(ErrorKind::InvalidData, WnfReadError::InvalidBitPattern))
        }
    }

    unsafe fn from_reader<F, Meta>(mut reader: F) -> io::Result<(Box<T>, Meta)>
    where
        F: FnMut(*mut c_void, usize) -> io::Result<(usize, Meta)>,
    {
        // Ideally, we would use `Box::new_uninit`, but that is unstable
        let mut bits = if mem::size_of::<T::Bits>() == 0 {
            // SAFETY:
            // The all-zero byte pattern is a valid `T::Bits` because `T::Bits` is zero-sized
            // (or, alternatively, because `T::Bits: AnyBitPattern`)
            Box::new(MaybeUninit::uninit())
        } else {
            let layout = Layout::new::<T::Bits>();

            // SAFETY:
            // `layout` has non-zero size
            let data = alloc::alloc(layout) as *mut MaybeUninit<T::Bits>;

            // SAFETY:
            // - `data` was allocated with the global allocator using the layout of `T::Bits`, which is the same as the
            //   layout of MaybeUninit<T::Bits>
            // - `data` is not aliased
            // - `data` points to a valid `MaybeUninit<T::Bits>` because a `MaybeUninit<_>` is always valid
            Box::from_raw(data)
        };

        // The precondition of `reader` is satisfied because `bits.as_mut_ptr()` is valid for writes of `T::Bits`
        let (size, meta) = reader(bits.as_mut_ptr().cast(), mem::size_of::<T::Bits>())?;

        if size != mem::size_of::<T::Bits>() {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                WnfReadError::WrongSize {
                    expected: mem::size_of::<T::Bits>(),
                    actual: size,
                },
            ));
        }

        // SAFETY:
        // - The raw pointer is obtained via `Box::into_raw` from a `Box<MaybeUninit<T::Bits>>`
        // - `T::Bits` has the same memory layout as `MaybeUninit<T::Bits>`
        // - The box contains a valid `T::Bits` because the memory range is initialized (by the safety condition)
        //   and `T::Bits: AnyBitPattern`
        let bits = Box::from_raw(Box::into_raw(bits) as *mut T::Bits);

        if T::is_valid_bit_pattern(&bits) {
            // SAFETY:
            // - The raw pointer is obtained via `Box::into_raw` from a `Box<T::Bits>`
            //
            // By the safety conditions of `CheckedBitPattern`,
            // - `T` has the same memory layout as `T::Bits`
            // - `bits` can be reinterpreted as a `T` because `T::is_valid_bit_pattern(&bits)` is `true`
            let data = Box::from_raw(Box::into_raw(bits) as *mut T);
            Ok((data, meta))
        } else {
            Err(io::Error::new(ErrorKind::InvalidData, WnfReadError::InvalidBitPattern))
        }
    }
}

impl<T> WnfRead<Box<[T]>> for [T]
where
    T: CheckedBitPattern,
{
    unsafe fn from_buffer(ptr: *const c_void, size: usize) -> io::Result<Box<[T]>> {
        if mem::size_of::<T::Bits>() == 0 {
            return if size == 0 {
                Ok(Vec::new().into_boxed_slice())
            } else {
                Err(io::Error::new(
                    ErrorKind::InvalidData,
                    WnfReadError::WrongSize {
                        expected: 0,
                        actual: size,
                    },
                ))
            };
        }

        if size % mem::size_of::<T::Bits>() != 0 {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                WnfReadError::WrongSizeMultiple {
                    expected_modulus: mem::size_of::<T::Bits>(),
                    actual: size,
                },
            ));
        }

        let len = size / mem::size_of::<T::Bits>();
        let mut buffer = Vec::with_capacity(len);

        // SAFETY:
        // - `ptr` is valid for reads of size `size` by the safety condition
        // - `buffer.as_mut_ptr()` is valid for writes of size `size` because
        //   `buffer.capacity() * mem::size_of::<T::Bits>() == size`
        // - Both `ptr` and `buffer.as_mut_ptr()` are trivially properly aligned as `mem::size_of::<u8>() == 1`
        // - The source and destination regions don't overlap because the source region is within the bounds of a
        //   single allocated object (because `ptr` is valid for reads) while the destination region is a freshly
        //   allocated object
        ptr::copy_nonoverlapping(ptr as *const u8, buffer.as_mut_ptr() as *mut u8, size);

        // SAFETY:
        // - `len <= buffer.capacity()`
        // - The elements at `0..len` are valid `T::Bits` because the memory range is initialized (by the safety
        //   condition) and `T::Bits: AnyBitPattern`
        buffer.set_len(len);

        if buffer.iter().all(T::is_valid_bit_pattern) {
            let data = buffer.into_boxed_slice();

            // SAFETY:
            // - The raw pointer is obtained via `Box::into_raw` from a `Box<[T::Bits]>`
            //
            // By the safety conditions of `CheckedBitPattern`,
            // - `T` has the same memory layout as `T::Bits`
            // - all elements of `data` can be reinterpreted as `T` because `T::is_valid_bit_pattern` is `true` for each
            //   element
            Ok(Box::from_raw(Box::into_raw(data) as *mut [T]))
        } else {
            Err(io::Error::new(ErrorKind::InvalidData, WnfReadError::InvalidBitPattern))
        }
    }

    unsafe fn from_reader<F, Meta>(mut reader: F) -> io::Result<(Box<[T]>, Meta)>
    where
        F: FnMut(*mut c_void, usize) -> io::Result<(usize, Meta)>,
    {
        let mut buffer: Vec<T::Bits> = Vec::new();

        // We need to loop to deal with race conditions caused by the WNF state data growing larger after we determine
        // its size but before we perform the actual read. This is guaranteed to terminate because we only reiterate
        // when the new size is strictly larger than the old one and there is an upper bound to the size of a WNF state
        let (len, meta) = loop {
            // The precondition of `reader` is satisfied because `buffer.as_mut_ptr()` is valid for writes of `T::Bits`
            let (size, meta) = reader(
                buffer.as_mut_ptr().cast(),
                buffer.capacity() * mem::size_of::<T::Bits>(),
            )?;

            if size == 0 {
                break (0, meta);
            }

            if mem::size_of::<T::Bits>() == 0 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    WnfReadError::WrongSize {
                        expected: 0,
                        actual: size,
                    },
                ));
            }

            if size % mem::size_of::<T::Bits>() != 0 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    WnfReadError::WrongSizeMultiple {
                        expected_modulus: mem::size_of::<T::Bits>(),
                        actual: size,
                    },
                ));
            }

            let len = size / mem::size_of::<T::Bits>();

            if len > buffer.capacity() {
                buffer.reserve(len - buffer.capacity());
                // Now `buffer.capacity() >= len`
            } else {
                break (len, meta);
            }
        };

        // SAFETY:
        // - `len <= buffer.capacity()`
        // - The elements at `0..len` are valid `T::Bits` because the memory range is initialized (by the safety
        //   condition) and `T::Bits: AnyBitPattern`
        buffer.set_len(len);

        if buffer.iter().all(T::is_valid_bit_pattern) {
            let data = buffer.into_boxed_slice();

            // SAFETY:
            // - The raw pointer is obtained via `Box::into_raw` from a `Box<[T::Bits]>`
            //
            // By the safety conditions of `CheckedBitPattern`,
            // - `T` has the same memory layout as `T::Bits`
            // - all elements of `data` can be reinterpreted as `T` because `T::is_valid_bit_pattern` is `true` for each
            //   element
            let data = Box::from_raw(Box::into_raw(data) as *mut [T]);

            Ok((data, meta))
        } else {
            Err(io::Error::new(ErrorKind::InvalidData, WnfReadError::InvalidBitPattern))
        }
    }
}

mod private {
    use super::*;

    pub trait Sealed {}

    impl Sealed for WnfOpaqueData {}
    impl<T> Sealed for T where T: CheckedBitPattern {}
    impl<T> Sealed for [T] where T: CheckedBitPattern {}
}

/// Error while reading WNF state data
#[derive(Debug, Error)]
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
