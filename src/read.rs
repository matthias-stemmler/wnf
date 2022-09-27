//! Reading types from WNF state data

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
    /// - The memory range of size `size` starting at `ptr` must be initialized
    unsafe fn from_buffer(ptr: *const c_void, size: usize) -> io::Result<D>;

    /// Tries to read a `D` by invoking a reader closure
    ///
    /// The reader closure takes a pointer to a buffer and the size of the buffer in bytes and tries to read a `D` into
    /// that buffer. It returns the actual number of bytes read and some metadata (such as a change stamp) that is
    /// passed through.
    ///
    /// # Safety
    /// When `reader` is invoked as `reader(ptr, size)`, it can assume that `ptr` is valid for accesses of size `size`
    ///
    /// When `reader(ptr, size)` returns `Ok((read_size, _))` with `read_size <= size`, then it must guarantee that the
    /// memory range of size `read_size` starting at `ptr` is initialized
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
        // The precondition of `reader` is satisfied because `NonNull::dangling()` is valid for zero-size accesses
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

        // The precondition of `reader` is satisfied because `bits.as_mut_ptr()` is valid for accesses of `T::Bits`
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
        // `bits.as_mut_ptr()` points to a valid `T::Bits` because the memory range is initialized (by the safety
        // condition and `size == mem::size_of::<T::Bits>()`) and `T::Bits: AnyBitPattern`
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
            // - Both `ptr` and `data` are trivially properly aligned as `mem::align_of::<u8>() == 1`
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
            Box::new(MaybeUninit::uninit())
        } else {
            let layout = Layout::new::<T::Bits>();

            // SAFETY:
            // `layout` has non-zero size
            let data = alloc::alloc(layout) as *mut MaybeUninit<T::Bits>;

            // SAFETY:
            // - `data` was allocated with the global allocator using the layout of `T::Bits`, which is the same as the
            //   layout of `MaybeUninit<T::Bits>`
            // - `data` is not aliased
            // - `data` points to a valid `MaybeUninit<T::Bits>` because a `MaybeUninit<_>` is always valid
            Box::from_raw(data)
        };

        // The precondition of `reader` is satisfied because `bits.as_mut_ptr()` is valid for accesses of `T::Bits`
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
        // - The box contains a valid `T::Bits` because the memory range is initialized (by the safety condition and
        //   `size == mem::size_of::<T::Bits>()`) and `T::Bits: AnyBitPattern`
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
        // - Both `ptr` and `buffer.as_mut_ptr()` are trivially properly aligned as `mem::align_of::<u8>() == 1`
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
            // The precondition of `reader` is satisfied because `buffer.as_mut_ptr()` is valid for accesses of
            // `T::Bits`
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
                buffer.reserve(len - buffer.len());
                // At this point we have `buffer.capacity() >= len`
            } else {
                break (len, meta);
            }
        };

        // At this point we have `size == len * mem::size_of::<T::Bits>()`

        // SAFETY:
        // - `len <= buffer.capacity()`
        // - The elements at `0..len` are valid `T::Bits` because the memory range is initialized (by the safety
        //   condition and `size == len * mem::size_of::<T::Bits>()`) and `T::Bits: AnyBitPattern`
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

/// Error reading WNF state data
#[derive(Debug, Error)]
pub enum WnfReadError {
    #[error("failed to read WNF state data: data has wrong size (expected {expected}, got {actual})")]
    WrongSize { expected: usize, actual: usize },

    #[error(
        "failed to read WNF state data: data has wrong size (expected a multiple of {expected_modulus}, got {actual})"
    )]
    WrongSizeMultiple { expected_modulus: usize, actual: usize },

    #[error("failed to read WNF state data: data has invalid bit pattern")]
    InvalidBitPattern,
}

mod private {
    use super::*;

    pub trait Sealed {}

    impl Sealed for WnfOpaqueData {}
    impl<T> Sealed for T where T: CheckedBitPattern {}
    impl<T> Sealed for [T] where T: CheckedBitPattern {}
}

#[cfg(test)]
mod tests {
    use std::cmp::min;

    use crate::AnyBitPattern;

    use super::*;

    #[test]
    fn opaque_data_from_buffer() {
        // SAFETY:
        // - `NonNull::dangling()` is valid for zero-size reads
        // - a zero-size memory range is always initialized
        let result = unsafe { WnfOpaqueData::from_buffer(NonNull::dangling().as_ptr(), 0) };

        assert!(result.is_ok());
    }

    #[test]
    fn opaque_data_from_reader() {
        // SAFETY: See `reader`
        let result = unsafe { WnfOpaqueData::from_reader(reader(&[0xFF; 2], "Meta")) };

        assert!(matches!(result, Ok((_, "Meta"))));
    }

    #[test]
    fn zero_sized_from_buffer_success() {
        // SAFETY:
        // - `NonNull::dangling()` is valid for zero-size reads
        // - a zero-size memory range is always initialized
        let result: io::Result<ZeroSized> = unsafe { ZeroSized::from_buffer(NonNull::dangling().as_ptr(), 0) };

        assert!(result.is_ok());
    }

    #[test]
    fn zero_sized_from_buffer_wrong_size() {
        let data = MisalignedU16::default();
        let (ptr, size) = data.as_buffer();

        // SAFETY:
        // - `ptr` and `size` come from a preallocated buffer
        let result: io::Result<ZeroSized> = unsafe { ZeroSized::from_buffer(ptr, size) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has wrong size (expected 0, got 2)"
        );
    }

    #[test]
    fn zero_sized_from_buffer_invalid_bit_pattern() {
        // SAFETY:
        // - `NonNull::dangling()` is valid for zero-size reads
        // - a zero-size memory range is always initialized
        let result: io::Result<AlwaysInvalid<ZeroSized>> =
            unsafe { AlwaysInvalid::<ZeroSized>::from_buffer(NonNull::dangling().as_ptr(), 0) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has invalid bit pattern"
        );
    }

    #[test]
    fn zero_sized_from_reader_success() {
        // SAFETY: See `reader`
        let result: io::Result<(ZeroSized, &str)> = unsafe { ZeroSized::from_reader(reader(&[], "Meta")) };

        assert!(matches!(result, Ok((_, "Meta"))));
    }

    #[test]
    fn zero_sized_from_reader_wrong_size() {
        // SAFETY: See `reader`
        let result: io::Result<(ZeroSized, &str)> = unsafe { ZeroSized::from_reader(reader(&[0xFF; 2], "Meta")) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has wrong size (expected 0, got 2)"
        );
    }

    #[test]
    fn zero_sized_from_reader_invalid_bit_pattern() {
        // SAFETY: See `reader`
        let result: io::Result<(AlwaysInvalid<ZeroSized>, &str)> =
            unsafe { AlwaysInvalid::<ZeroSized>::from_reader(reader(&[], "Meta")) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has invalid bit pattern"
        );
    }

    #[test]
    fn zero_sized_boxed_from_buffer_success() {
        // SAFETY:
        // - `NonNull::dangling()` is valid for zero-size reads
        // - a zero-size memory range is always initialized
        let result: io::Result<Box<ZeroSized>> = unsafe { ZeroSized::from_buffer(NonNull::dangling().as_ptr(), 0) };

        assert!(result.is_ok());
    }

    #[test]
    fn zero_sized_boxed_from_reader_success() {
        // SAFETY: See `reader`
        let result: io::Result<(Box<ZeroSized>, &str)> = unsafe { ZeroSized::from_reader(reader(&[], "Meta")) };

        assert!(matches!(result, Ok((_, "Meta"))));
    }

    #[test]
    fn zero_sized_slice_from_buffer_success() {
        // SAFETY:
        // - `NonNull::dangling()` is valid for zero-size reads
        // - a zero-size memory range is always initialized
        let result: io::Result<Box<[ZeroSized]>> =
            unsafe { <[ZeroSized]>::from_buffer(NonNull::dangling().as_ptr(), 0) };

        assert!(matches!(result, Ok(read_data) if read_data.is_empty()));
    }

    #[test]
    fn zero_sized_slice_from_buffer_wrong_size() {
        let data = MisalignedU16::default();
        let (ptr, size) = data.as_buffer();

        // SAFETY:
        // - `ptr` and `size` come from a preallocated buffer
        let result: io::Result<Box<[ZeroSized]>> = unsafe { <[ZeroSized]>::from_buffer(ptr, size) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has wrong size (expected 0, got 2)"
        );
    }

    #[test]
    fn zero_sized_slice_from_reader_success() {
        // SAFETY: See `reader`
        let result: io::Result<(Box<[ZeroSized]>, &str)> = unsafe { <[ZeroSized]>::from_reader(reader(&[], "Meta")) };

        assert!(matches!(result, Ok((read_data, "Meta")) if read_data.is_empty()));
    }

    #[test]
    fn zero_sized_slice_from_reader_wrong_size() {
        // SAFETY: See `reader`
        let result: io::Result<(Box<[ZeroSized]>, &str)> =
            unsafe { <[ZeroSized]>::from_reader(reader(&[0xFF; 2], "Meta")) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has wrong size (expected 0, got 2)"
        );
    }

    #[test]
    fn nonzero_sized_from_buffer_success() {
        let data = MisalignedU16::default();
        let (ptr, size) = data.as_buffer();

        // SAFETY:
        // - `ptr` and `size` come from a preallocated buffer
        let result: io::Result<u16> = unsafe { u16::from_buffer(ptr, size) };

        assert!(matches!(result, Ok(read_data) if read_data == data.as_u16()));
    }

    #[test]
    fn nonzero_sized_from_buffer_wrong_size() {
        let data = MisalignedU16::default();
        let (ptr, size) = data.as_buffer();

        // SAFETY:
        // - `ptr` and `size` come from a preallocated buffer
        let result: io::Result<u32> = unsafe { u32::from_buffer(ptr, size) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has wrong size (expected 4, got 2)"
        );
    }

    #[test]
    fn nonzero_sized_from_buffer_invalid_bit_pattern() {
        let data = MisalignedU16::default();
        let (ptr, size) = data.as_buffer();

        // SAFETY:
        // - `ptr` and `size` come from a preallocated buffer
        let result: io::Result<AlwaysInvalid<u16>> = unsafe { AlwaysInvalid::<u16>::from_buffer(ptr, size) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has invalid bit pattern"
        );
    }

    #[test]
    fn nonzero_sized_from_reader_success() {
        let data: u16 = 0x1234;

        // SAFETY: See `reader`
        let result: io::Result<(u16, &str)> = unsafe { u16::from_reader(reader(&data.to_le_bytes(), "Meta")) };

        assert!(matches!(result, Ok((read_data, "Meta")) if read_data == data));
    }

    #[test]
    fn nonzero_sized_from_reader_wrong_size() {
        // SAFETY: See `reader`
        let result: io::Result<(u32, &str)> = unsafe { u32::from_reader(reader(&[0xFF; 2], "Meta")) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has wrong size (expected 4, got 2)"
        );
    }

    #[test]
    fn nonzero_sized_from_reader_invalid_bit_pattern() {
        // SAFETY: See `reader`
        let result: io::Result<(AlwaysInvalid<u16>, &str)> =
            unsafe { AlwaysInvalid::<u16>::from_reader(reader(&[0xFF; 2], "Meta")) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has invalid bit pattern"
        );
    }

    #[test]
    fn nonzero_sized_boxed_from_buffer_success() {
        let data = MisalignedU16::default();
        let (ptr, size) = data.as_buffer();

        // SAFETY:
        // - `ptr` and `size` come from a preallocated buffer
        let result: io::Result<Box<u16>> = unsafe { u16::from_buffer(ptr, size) };

        assert!(matches!(result, Ok(read_data) if *read_data == data.as_u16()));
    }

    #[test]
    fn nonzero_sized_boxed_from_reader_success() {
        let data: u16 = 0x1234;

        // SAFETY: See `reader`
        let result: io::Result<(Box<u16>, &str)> = unsafe { u16::from_reader(reader(&data.to_le_bytes(), "Meta")) };

        assert!(matches!(result, Ok((read_data, "Meta")) if *read_data == data));
    }

    #[test]
    fn nonzero_sized_slice_from_buffer_success_empty() {
        // SAFETY:
        // - `NonNull::dangling()` is valid for zero-size reads
        // - a zero-size memory range is always initialized
        let result: io::Result<Box<[u16]>> = unsafe { <[u16]>::from_buffer(NonNull::dangling().as_ptr(), 0) };

        assert!(matches!(result, Ok(read_data) if read_data.is_empty()));
    }

    #[test]
    fn nonzero_sized_slice_from_buffer_success_nonempty() {
        let data = MisalignedU16Slice::default();
        let (ptr, size) = data.as_buffer();

        // SAFETY:
        // - `ptr` and `size` come from a preallocated buffer
        let result: io::Result<Box<[u16]>> = unsafe { <[u16]>::from_buffer(ptr, size) };

        assert!(matches!(result, Ok(read_data) if *read_data == *data.as_u16_slice()));
    }

    #[test]
    fn nonzero_sized_slice_from_buffer_wrong_size_multiple() {
        let data = MisalignedU16Slice::default();
        let (ptr, size) = data.as_buffer();

        // SAFETY:
        // - `ptr` and `size` come from a preallocated buffer
        let result: io::Result<Box<[u64]>> = unsafe { <[u64]>::from_buffer(ptr, size) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has wrong size (expected a multiple of 8, got 4)"
        );
    }

    #[test]
    fn nonzero_sized_slice_from_buffer_invalid_bit_pattern() {
        let data = MisalignedU16Slice::default();
        let (ptr, size) = data.as_buffer();

        // SAFETY:
        // - `ptr` and `size` come from a preallocated buffer
        let result: io::Result<Box<[AlwaysInvalid<u16>]>> = unsafe { <[AlwaysInvalid<u16>]>::from_buffer(ptr, size) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has invalid bit pattern"
        );
    }

    #[test]
    fn nonzero_sized_slice_from_reader_success() {
        let data: [u16; 2] = [0x1234, 0x5678];
        let raw_data: Vec<_> = data.iter().flat_map(|&value| value.to_le_bytes().into_iter()).collect();

        // SAFETY: See `reader`
        let result: io::Result<(Box<[u16]>, &str)> = unsafe { <[u16]>::from_reader(reader(&raw_data, "Meta")) };

        assert!(matches!(result, Ok((read_data, "Meta")) if *read_data == data));
    }

    #[test]
    fn nonzero_sized_slice_from_reader_growing() {
        // We pass a reader that first produces one element, then five
        // After seeing the one element, the destination vector will grow directly to a capacity of four elements, so it
        // needs to grow by one more element
        let data: [u16; 5] = [0x1122, 0x3344, 0x5566, 0x7788, 0x99AA];
        let raw_data_1 = data[0].to_le_bytes();
        let raw_data_2: Vec<_> = data.iter().flat_map(|&value| value.to_le_bytes().into_iter()).collect();

        // SAFETY: See `multireader`
        let result: io::Result<(Box<[u16]>, &str)> =
            unsafe { <[u16]>::from_reader(multireader(vec![(&raw_data_1, "Meta 1"), (&raw_data_2, "Meta 2")])) };

        assert!(matches!(result, Ok((read_data, "Meta 2")) if *read_data == data));
    }

    #[test]
    fn nonzero_sized_slice_from_reader_wrong_size_multiple() {
        // SAFETY: See `reader`
        let result: io::Result<(Box<[u64]>, &str)> = unsafe { <[u64]>::from_reader(reader(&[0xFF; 4], "Meta")) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has wrong size (expected a multiple of 8, got 4)"
        );
    }

    #[test]
    fn nonzero_sized_slice_from_reader_invalid_bit_pattern() {
        // SAFETY: See `reader`
        let result: io::Result<(Box<[AlwaysInvalid<u16>]>, &str)> =
            unsafe { <[AlwaysInvalid<u16>]>::from_reader(reader(&[0xFF; 4], "Meta")) };

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "failed to read WNF state data: data has invalid bit pattern"
        );
    }

    #[derive(Clone, Copy, Debug)]
    #[repr(C)]
    struct ZeroSized;

    // SAFETY:
    // `ZeroSized` is zero-sized
    unsafe impl AnyBitPattern for ZeroSized {}

    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    struct AlwaysInvalid<T>(T);

    // SAFETY:
    // `AlwaysInvalid<T>` has the same memory layout as `T`
    // `is_valid_bit_pattern` always returns `false`
    unsafe impl<T> CheckedBitPattern for AlwaysInvalid<T>
    where
        T: AnyBitPattern,
    {
        type Bits = T;

        fn is_valid_bit_pattern(_: &T) -> bool {
            false
        }
    }

    #[derive(Debug, Default)]
    struct MisalignedU16 {
        slice: MisalignedU16Slice,
    }

    impl MisalignedU16 {
        fn as_u16(&self) -> u16 {
            self.slice.as_u16_slice()[0]
        }

        fn as_buffer(&self) -> (*const c_void, usize) {
            let (ptr, size) = self.slice.as_buffer();
            (ptr, size / self.slice.len())
        }
    }

    #[derive(Debug)]
    struct MisalignedU16Slice {
        buffer: [u16; 3],
    }

    impl Default for MisalignedU16Slice {
        fn default() -> Self {
            Self {
                // Since `mem::align_of::<[u16; 3]>() == 2`, the buffer looks like this in memory
                // (assuming little-endian byte order):
                //
                // Memory address mod 2 | 0    | 1    | 0    | 1    | 0    | 1    |
                // Value                | 0xFF | 0x34 | 0x12 | 0x78 | 0x56 | 0xFF |
                //
                // So the 4-byte subslice starting at an offset of 1 byte is a misaligned `[u16]` with value
                // `[0x1234, 0x5678]`
                buffer: [0x34FF, 0x7812, 0xFF56],
            }
        }
    }

    impl MisalignedU16Slice {
        fn len(&self) -> usize {
            self.as_u16_slice().len()
        }

        fn as_u16_slice(&self) -> &[u16] {
            &[0x1234, 0x5678]
        }

        fn as_buffer(&self) -> (*const c_void, usize) {
            let ptr = self.buffer.as_ptr() as *const u8;

            // SAFETY:
            // - Both `ptr` and the offset pointer are in bounds of the same allocated object `self.buffer`
            // - The computed offset does not overflow an `isize`
            // - The computed sum does not overflow a `usize`
            let ptr = unsafe { ptr.offset(1) };

            assert_ne!((ptr as usize) % mem::align_of::<u16>(), 0);
            (ptr as *const c_void, self.len() * mem::size_of::<u16>())
        }
    }

    /// Produces a reader that reads from the provided data and returns the provided metadata
    fn reader<'a, Meta>(data: &'a [u8], meta: Meta) -> impl FnMut(*mut c_void, usize) -> io::Result<(usize, Meta)> + 'a
    where
        Meta: Clone + 'a,
    {
        multireader(vec![(data, meta)])
    }

    /// Produces a reader that, on its first invocation, reads from the provided data and returns the provided metadata
    /// from the first element of `outputs`, then on its second invocation from the second element and so on
    ///
    /// When `outputs` is exhausted, its last element is repeated.
    ///
    /// # Safety notes
    /// - `multireader(&[(data, _), ..])(ptr, size)` returns `Ok((data.len(), _))`
    /// - If `data.len() <= size`, then `min(data.len(), size) == data.len()` and the memory range of size `data.len()`
    ///   starting at `ptr` is initialized because it is filled by `ptr::copy_nonoverlapping`
    fn multireader<'a, Meta>(
        outputs: Vec<(&'a [u8], Meta)>,
    ) -> impl FnMut(*mut c_void, usize) -> io::Result<(usize, Meta)> + 'a
    where
        Meta: Clone + 'a,
    {
        let mut outputs = (0..).map(move |idx| outputs[min(idx, outputs.len() - 1)].clone());

        move |ptr, size| {
            let (data, meta) = outputs.next().unwrap();

            // SAFETY:
            // - `data.as_ptr()` is valid for reads of `min(data.len(), size)` because it comes from the live reference
            //   `data` pointing to data of size `data.len()`
            // - `ptr` is valid for writes of `min(data.len(), size)` by the safety conditions of `from_reader`
            // - Both `data.as_ptr()` and `ptr` are trivially properly aligned as `mem::align_of::<u8>() == 1`
            // - The source and destination regions don't overlap because the source region is within the bounds of the
            //   single allocated object `data` while the destination region is within the bounds of another single
            //   allocated object because `ptr` is valid for writes
            unsafe {
                ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, min(data.len(), size));
            }

            Ok((data.len(), meta))
        }
    }
}
