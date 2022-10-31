#![allow(clippy::undocumented_unsafe_blocks)]

use std::marker::{PhantomData, PhantomPinned};
use std::mem::ManuallyDrop;
use std::num;

/// # Safety
/// It is safe to implement this trait for a type `T` if and only if any initialized sequence of bytes ("bit pattern")
/// with the same layout (i.e. size and alignment) as `T` can be interpreted as a value of type `T`
pub unsafe trait AnyBitPattern: Copy + Send + Sized + 'static {}

// SAFETY: Any bit pattern is valid for these primitive types
unsafe impl AnyBitPattern for () {}
unsafe impl AnyBitPattern for u8 {}
unsafe impl AnyBitPattern for i8 {}
unsafe impl AnyBitPattern for u16 {}
unsafe impl AnyBitPattern for i16 {}
unsafe impl AnyBitPattern for u32 {}
unsafe impl AnyBitPattern for i32 {}
unsafe impl AnyBitPattern for u64 {}
unsafe impl AnyBitPattern for i64 {}
unsafe impl AnyBitPattern for u128 {}
unsafe impl AnyBitPattern for i128 {}
unsafe impl AnyBitPattern for usize {}
unsafe impl AnyBitPattern for isize {}
unsafe impl AnyBitPattern for f32 {}
unsafe impl AnyBitPattern for f64 {}

// SAFETY:
// - `NonZero*` are `#[repr(transparent)]` wrappers around the corresponding primitive types
// - Niche optimization for `Option<NonZero*>` is guaranteed (see https://doc.rust-lang.org/std/option/#representation)
unsafe impl AnyBitPattern for Option<num::NonZeroU8> {}
unsafe impl AnyBitPattern for Option<num::NonZeroI8> {}
unsafe impl AnyBitPattern for Option<num::NonZeroU16> {}
unsafe impl AnyBitPattern for Option<num::NonZeroI16> {}
unsafe impl AnyBitPattern for Option<num::NonZeroU32> {}
unsafe impl AnyBitPattern for Option<num::NonZeroI32> {}
unsafe impl AnyBitPattern for Option<num::NonZeroU64> {}
unsafe impl AnyBitPattern for Option<num::NonZeroI64> {}
unsafe impl AnyBitPattern for Option<num::NonZeroU128> {}
unsafe impl AnyBitPattern for Option<num::NonZeroI128> {}
unsafe impl AnyBitPattern for Option<num::NonZeroUsize> {}
unsafe impl AnyBitPattern for Option<num::NonZeroIsize> {}

// SAFETY: `ManuallyDrop<T>` is a `#[repr(transparent)]` wrapper around `T`
unsafe impl<T> AnyBitPattern for ManuallyDrop<T> where T: AnyBitPattern {}

// SAFETY: `PhantomData<T>` is zero-sized
unsafe impl<T> AnyBitPattern for PhantomData<T> where T: Send + 'static + ?Sized {}

// SAFETY: `PhantomPinned` is zero-sized
unsafe impl AnyBitPattern for PhantomPinned {}

// SAFETY: `num::Wrapping<T>` is a `#[repr(transparent)]` wrapper around `T`
unsafe impl<T> AnyBitPattern for num::Wrapping<T> where T: AnyBitPattern {}

// SAFETY: Each array element can be interpreted as a value of type `T`
unsafe impl<T, const N: usize> AnyBitPattern for [T; N] where T: AnyBitPattern {}

/// # Safety
/// An implementation of this trait for a type `T` is safe if and only the following hold:
/// - `<T as CheckedBitPattern>::Bits` has the same memory layout (i.e. size and alignment) as `T`
/// - Any value `bits: <T as CheckedBitPattern>::Bits` for which `<T as CheckedBitPattern>::is_valid_bit_pattern(&bits)`
///   is `true` can be interpreted as a value of type `T`
pub unsafe trait CheckedBitPattern: Copy + Send + Sized + 'static {
    type Bits: AnyBitPattern;

    fn is_valid_bit_pattern(bits: &Self::Bits) -> bool;
}

// SAFETY:
// - `<T as CheckedBitPattern>::Bits` trivially has the same layout as `T` because it *is* `T`
// - any value of type `<T as CheckedBitPattern>::Bits` can trivially be interpreted as a value of type `T` because it
//   *is* a value of type `T`
unsafe impl<T> CheckedBitPattern for T
where
    T: AnyBitPattern,
{
    type Bits = T;

    #[inline(always)]
    fn is_valid_bit_pattern(_bits: &T) -> bool {
        true
    }
}

// SAFETY: see `char::from_u32`
unsafe impl CheckedBitPattern for char {
    type Bits = u32;

    #[inline]
    fn is_valid_bit_pattern(bits: &u32) -> bool {
        char::from_u32(*bits).is_some()
    }
}

// SAFETY: see https://doc.rust-lang.org/reference/types/boolean.html#boolean-type
unsafe impl CheckedBitPattern for bool {
    type Bits = u8;

    #[inline]
    fn is_valid_bit_pattern(bits: &u8) -> bool {
        matches!(*bits, 0 | 1)
    }
}

/// # Safety
/// It is safe to implement this trait for a type `T` if and only if the memory representation of any value of type `T`
/// contains no uninitialized (i.e. padding) bytes
pub unsafe trait NoUninit {}

// SAFETY: Values of these primitive types contain no uninitialized bytes
unsafe impl NoUninit for () {}
unsafe impl NoUninit for u8 {}
unsafe impl NoUninit for i8 {}
unsafe impl NoUninit for u16 {}
unsafe impl NoUninit for i16 {}
unsafe impl NoUninit for u32 {}
unsafe impl NoUninit for i32 {}
unsafe impl NoUninit for u64 {}
unsafe impl NoUninit for i64 {}
unsafe impl NoUninit for u128 {}
unsafe impl NoUninit for i128 {}
unsafe impl NoUninit for usize {}
unsafe impl NoUninit for isize {}
unsafe impl NoUninit for f32 {}
unsafe impl NoUninit for f64 {}
unsafe impl NoUninit for char {}
unsafe impl NoUninit for bool {}

// SAFETY: `NonZero*` are `#[repr(transparent)]` wrappers around the corresponding primitive types
unsafe impl NoUninit for num::NonZeroU8 {}
unsafe impl NoUninit for num::NonZeroI8 {}
unsafe impl NoUninit for num::NonZeroU16 {}
unsafe impl NoUninit for num::NonZeroI16 {}
unsafe impl NoUninit for num::NonZeroU32 {}
unsafe impl NoUninit for num::NonZeroI32 {}
unsafe impl NoUninit for num::NonZeroU64 {}
unsafe impl NoUninit for num::NonZeroI64 {}
unsafe impl NoUninit for num::NonZeroU128 {}
unsafe impl NoUninit for num::NonZeroI128 {}
unsafe impl NoUninit for num::NonZeroUsize {}
unsafe impl NoUninit for num::NonZeroIsize {}

// SAFETY:
// - `NonZero*` are `#[repr(transparent)]` wrappers around the corresponding primitive types
// - Niche optimization for `Option<NonZero*>` is guaranteed (see https://doc.rust-lang.org/std/option/#representation)
unsafe impl NoUninit for Option<num::NonZeroU8> {}
unsafe impl NoUninit for Option<num::NonZeroI8> {}
unsafe impl NoUninit for Option<num::NonZeroU16> {}
unsafe impl NoUninit for Option<num::NonZeroI16> {}
unsafe impl NoUninit for Option<num::NonZeroU32> {}
unsafe impl NoUninit for Option<num::NonZeroI32> {}
unsafe impl NoUninit for Option<num::NonZeroU64> {}
unsafe impl NoUninit for Option<num::NonZeroI64> {}
unsafe impl NoUninit for Option<num::NonZeroU128> {}
unsafe impl NoUninit for Option<num::NonZeroI128> {}
unsafe impl NoUninit for Option<num::NonZeroUsize> {}
unsafe impl NoUninit for Option<num::NonZeroIsize> {}

// SAFETY: `ManuallyDrop<T>` is a `#[repr(transparent)]` wrapper around `T`
unsafe impl<T> NoUninit for ManuallyDrop<T> where T: NoUninit + ?Sized {}

// SAFETY: `PhantomData<T>` is zero-sized
unsafe impl<T> NoUninit for PhantomData<T> where T: ?Sized {}

// SAFETY: `PhantomPinned` is zero-sized
unsafe impl NoUninit for PhantomPinned {}

// SAFETY: `num::Wrapping<T>` is a `#[repr(transparent)]` wrapper around `T`
unsafe impl<T> NoUninit for num::Wrapping<T> where T: NoUninit {}

// SAFETY:
// - Each array element contains no uninitialized bytes
// - There is no padding between the elements
unsafe impl<T, const N: usize> NoUninit for [T; N] where T: NoUninit {}

// SAFETY:
// - Each slice element contains no uninitialized bytes
// - There is no padding between the elements
unsafe impl<T> NoUninit for [T] where T: NoUninit {}

pub mod reexports {
    #[cfg(feature = "bytemuck_v1")]
    pub mod bytemuck {
        pub use bytemuck_v1 as v1;
    }

    #[cfg(feature = "zerocopy")]
    pub use zerocopy;
}

#[cfg(feature = "bytemuck_v1")]
#[macro_export]
macro_rules! derive_from_bytemuck_v1 {
    (AnyBitPattern for $type:ty) => {
        const _: fn() = || {
            use $crate::reexports::bytemuck::v1 as bytemuck_v1;

            fn assert_impl_any_bit_pattern<T: ?Sized + bytemuck_v1::AnyBitPattern>() {}
            assert_impl_any_bit_pattern::<$type>();

            // SAFETY:
            // - the above asserts that $type : bytemuck_v1::AnyBitPattern
            // - this implies the safety conditions of wnf::AnyBitPattern
            unsafe impl $crate::AnyBitPattern for $type {}
        };
    };

    (CheckedBitPattern for $type:ty) => {
        const _: fn() = || {
            use $crate::reexports::bytemuck::v1 as bytemuck_v1;

            fn assert_impl_checked_bit_pattern<T: ?Sized + bytemuck_v1::CheckedBitPattern>() {}
            assert_impl_checked_bit_pattern::<$type>();

            // SAFETY:
            // - the implementation just delegates to bytemuck_v1::CheckedBitPattern
            // - this implies the safety conditions of wnf::CheckedBitPattern
            unsafe impl $crate::CheckedBitPattern for $type {
                type Bits = <$type as bytemuck_v1::CheckedBitPattern>::Bits;

                fn is_valid_bit_pattern(bits: &Self::Bits) -> bool {
                    <$type as bytemuck_v1::CheckedBitPattern>::is_valid_bit_pattern(bits)
                }
            }
        };
    };

    (NoUninit for $type:ty) => {
        const _: fn() = || {
            use $crate::reexports::bytemuck::v1 as bytemuck_v1;

            fn assert_impl_no_uninit<T: ?Sized + bytemuck_v1::NoUninit>() {}
            assert_impl_no_uninit::<$type>();

            // SAFETY:
            // - the above asserts that $type : bytemuck_v1::NoUninit
            // - this implies the safety conditions of wnf::NoUninit
            unsafe impl $crate::NoUninit for $type {}
        };
    };

    ($trait:ident for $type:ty) => {
        compile_error!(concat!(
            "Trait must be one of AnyBitPattern, CheckedBitPattern, NoUninit (found: ",
            stringify!($trait),
            ")"
        ));
    };
}

#[cfg(feature = "zerocopy")]
#[macro_export]
macro_rules! derive_from_zerocopy {
    (AnyBitPattern for $type:ty) => {
        const _: fn() = || {
            use $crate::reexports::zerocopy;

            fn assert_impl_from_bytes<T: ?Sized + zerocopy::FromBytes>() {}
            assert_impl_from_bytes::<$type>();

            // SAFETY:
            // - the above asserts that $type : zerocopy::FromBytes
            // - this implies the safety conditions of wnf::AnyBitPattern
            unsafe impl $crate::AnyBitPattern for $type {}
        };
    };

    (CheckedBitPattern for $type:ty) => {
        compile_error!("CheckedBitPattern cannot by derived from zerocopy");
    };

    (NoUninit for $type:ty) => {
        const _: fn() = || {
            fn assert_impl_as_bytes<T: ?Sized + zerocopy::AsBytes>() {}
            assert_impl_as_bytes::<$type>();

            // SAFETY:
            // - the above asserts that $type : zerocopy::AsBytes
            // - this implies the safety conditions of wnf::NoUninit
            unsafe impl $crate::NoUninit for $type {}
        };
    };

    ($trait:ident for $type:ty) => {
        compile_error!(concat!(
            "Trait must be one of AnyBitPattern, NoUninit (found: ",
            stringify!($trait),
            ")"
        ));
    };
}
