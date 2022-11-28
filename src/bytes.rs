//! Traits for casting between plain data types

// This is needed because one safety comment applies to multiple unsafe impls
#![allow(clippy::undocumented_unsafe_blocks)]

use std::marker::{PhantomData, PhantomPinned};
use std::mem::ManuallyDrop;
use std::num;

/// Marker trait for types for which any bit pattern is valid
///
/// This is modelled after the [`AnyBitPattern`](bytemuck_v1::AnyBitPattern) trait of the [`bytemuck`](bytemuck_v1)
/// crate.
///
/// In order for reading a value of a type `T` from a WNF state to be sound, `T` is required to implement
/// [`AnyBitPattern`] (or at least [`CheckedBitPattern`], which is implied by [`AnyBitPattern`]).
///
/// # Implementation
/// This trait is already implemented by the [`wnf`](crate) crate for many primitive types and types from the standard
/// library. There are several ways to implement it for your own types:
/// - Implement it directly, requiring `unsafe` code
/// - Derive the [`AnyBitPattern`](bytemuck_v1::AnyBitPattern) trait of the [`bytemuck`](bytemuck_v1) crate and derive
///   this trait from it via the [`derive_from_bytemuck_v1`](crate::derive_from_bytemuck_v1) macro:
/// ```
/// # #[macro_use] extern crate wnf;
/// # extern crate bytemuck_v1 as bytemuck;
/// #
/// # fn main() {
/// use wnf::{derive_from_bytemuck_v1, OwnedState};
///
/// #[derive(bytemuck::AnyBitPattern, bytemuck::NoUninit, Copy, Clone)]
/// #[repr(transparent)]
/// struct MyType(u32);
///
/// derive_from_bytemuck_v1!(AnyBitPattern for MyType);
/// derive_from_bytemuck_v1!(NoUninit for MyType);
///
/// let state: OwnedState<MyType> = OwnedState::create_temporary().expect("Failed to create state");
/// state.set(&MyType(42)).expect("Failed to update state data");
/// let data = state.get().expect("Failed to query state data");
///
/// assert_eq!(data.0, 42);
/// # }
/// ```
/// - Derive the [`AsBytes`](zerocopy::AsBytes) trait of the [`zerocopy`] crate and derive this trait from it via the
///   [`derive_from_zerocopy`](crate::derive_from_zerocopy) macro:
/// ```
/// # #[macro_use] extern crate wnf;
/// #
/// # fn main() {
/// use wnf::{derive_from_zerocopy, OwnedState};
///
/// #[derive(zerocopy::AsBytes, zerocopy::FromBytes, Copy, Clone)]
/// #[repr(transparent)]
/// struct MyType(u32);
///
/// derive_from_zerocopy!(AnyBitPattern for MyType);
/// derive_from_zerocopy!(NoUninit for MyType);
///
/// let state: OwnedState<MyType> = OwnedState::create_temporary().expect("Failed to create state");
/// state.set(&MyType(42)).expect("Failed to update state data");
/// let data = state.get().expect("Failed to query state data");
///
/// assert_eq!(data.0, 42);
/// # }
/// ```
///
/// # Safety
/// Implementing this trait for a type `T` is sound if any initialized sequence of bytes ("bit pattern") with the same
/// layout (i.e. size and alignment) as `T` is a valid `T`.
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

// SAFETY: Each array element is a valid `T`
unsafe impl<T, const N: usize> AnyBitPattern for [T; N] where T: AnyBitPattern {}

/// Trait for types that can be checked for valid bit patterns at runtime
///
/// This is modelled after the [`CheckedBitPattern`](bytemuck_v1::CheckedBitPattern) trait of the
/// [`bytemuck`](bytemuck_v1) crate.
///
/// In order for reading a value of a type `T` from a WNF state to be sound, `T` is required to implement
/// [`CheckedBitPattern`] (or [`AnyBitPattern`], which imples [`CheckedBitPattern`]).
///
/// In case *any* bit pattern is valid for a type, implement the [`AnyBitPattern`] trait instead.
///
/// # Implementation
/// This trait is already implemented by the [`wnf`](crate) crate for many primitive types and types from the standard
/// library. There are several ways to implement it for your own types:
/// - Implement it directly, requiring `unsafe` code
/// - Derive the [`CheckedBitPattern`](bytemuck_v1::CheckedBitPattern) trait of the [`bytemuck`](bytemuck_v1) crate and
///   derive this trait from it via the [`derive_from_bytemuck_v1`](crate::derive_from_bytemuck_v1) macro:
/// ```
/// # #[macro_use] extern crate wnf;
/// # extern crate bytemuck_v1 as bytemuck;
/// #
/// # fn main() {
/// use wnf::{derive_from_bytemuck_v1, OwnedState};
///
/// #[derive(bytemuck::CheckedBitPattern, bytemuck::NoUninit, Copy, Clone)]
/// #[repr(transparent)]
/// struct MyType(bool);
///
/// derive_from_bytemuck_v1!(CheckedBitPattern for MyType);
/// derive_from_bytemuck_v1!(NoUninit for MyType);
///
/// let state: OwnedState<MyType> = OwnedState::create_temporary().expect("Failed to create state");
/// state.set(&MyType(true)).expect("Failed to update state data");
/// let data = state.get().expect("Failed to query state data");
///
/// assert!(data.0);
/// # }
/// ```
///
/// # Safety
/// Implementing this trait for a type `T` is sound if:
/// - `<T as CheckedBitPattern>::Bits` has the same memory layout (i.e. size and alignment) as `T`
/// - Any value `bits: <T as CheckedBitPattern>::Bits` for which `<T as CheckedBitPattern>::is_valid_bit_pattern(&bits)`
///   is `true` can be interpreted as a valid `T`
pub unsafe trait CheckedBitPattern: Copy + Send + Sized + 'static {
    /// The type of the underlying bit patterns that can be checked for validity
    type Bits: AnyBitPattern;

    /// Checks whether the given bit pattern can be interpreted as a valid `Self`
    fn is_valid_bit_pattern(bits: &Self::Bits) -> bool;
}

// SAFETY:
// - `<T as CheckedBitPattern>::Bits` trivially has the same memory layout as `T` because it *is* `T`
// - any value `bits: <T as CheckedBitPattern>::Bits` can trivially be interpreted as a valid `T` because it *is* a
//   valid `T`
unsafe impl<T> CheckedBitPattern for T
where
    T: AnyBitPattern,
{
    type Bits = T;

    #[inline]
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

/// Marker trait for types without uninitialized (padding) bytes
///
/// This is modelled after the [`NoUninit`](bytemuck_v1::NoUninit) trait of the [`bytemuck`](bytemuck_v1) crate.
///
/// In order for writing a value of a type `T` to a WNF state to be sound, `T` is required to implement [`NoUninit`].
///
/// # Implementation
/// This trait is already implemented by the [`wnf`](crate) crate for many primitive types and types from the standard
/// library. There are several ways to implement it for your own types:
/// - Implement it directly, requiring `unsafe` code
/// - Derive the [`NoUninit`](bytemuck_v1::NoUninit) trait of the [`bytemuck`](bytemuck_v1) crate and derive this trait
///   from it via the [`derive_from_bytemuck_v1`](crate::derive_from_bytemuck_v1) macro:
/// ```
/// # #[macro_use] extern crate wnf;
/// # extern crate bytemuck_v1 as bytemuck;
/// #
/// # fn main() {
/// use wnf::{derive_from_bytemuck_v1, OwnedState};
///
/// #[derive(bytemuck::NoUninit, bytemuck::AnyBitPattern, Copy, Clone)]
/// #[repr(transparent)]
/// struct MyType(u32);
///
/// derive_from_bytemuck_v1!(NoUninit for MyType);
/// derive_from_bytemuck_v1!(AnyBitPattern for MyType);
///
/// let state: OwnedState<MyType> = OwnedState::create_temporary().expect("Failed to create state");
/// state.set(&MyType(42)).expect("Failed to update state data");
/// let data = state.get().expect("Failed to query state data");
///
/// assert_eq!(data.0, 42);
/// # }
/// ```
/// - Derive the [`FromBytes`](zerocopy::FromBytes) trait of the [`zerocopy`] crate and derive this trait from it via
///   the [`derive_from_zerocopy`](crate::derive_from_zerocopy) macro:
/// ```
/// # #[macro_use] extern crate wnf;
/// #
/// # fn main() {
/// use wnf::{derive_from_zerocopy, OwnedState};
///
/// #[derive(zerocopy::FromBytes, zerocopy::AsBytes, Copy, Clone)]
/// #[repr(transparent)]
/// struct MyType(u32);
///
/// derive_from_zerocopy!(NoUninit for MyType);
/// derive_from_zerocopy!(AnyBitPattern for MyType);
///
/// let state: OwnedState<MyType> = OwnedState::create_temporary().expect("Failed to create state");
/// state.set(&MyType(42)).expect("Failed to update state data");
/// let data = state.get().expect("Failed to query state data");
///
/// assert_eq!(data.0, 42);
/// # }
/// ```
///
/// # Safety
/// Implementing this trait for a type `T` is sound if the memory representation of any `T` contains no uninitialized
/// (i.e. padding) bytes.
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

/// Reexports of items from third-party crates for use in macro-generated code
#[doc(hidden)]
pub mod __reexports {
    /// Reexport of the [`bytemuck`](bytemuck_v1) crate in version 1.x
    ///
    /// This will be referred to by macro-generated code as `wnf::__reexports::bytemuck::v1`. The `v1` is for forward
    /// compatibility in case there will ever be a major version `2.x` of `bytemuck`. In that case, items from `v1`
    /// and `v2` are distinct and need to be reexported separately.
    #[cfg(feature = "bytemuck_v1")]
    pub mod bytemuck {
        pub use bytemuck_v1 as v1;
    }

    /// Reexport of the [`zerocopy`] crate
    ///
    /// This will be referred to by macro-generated code as `wnf::__reexports::zerocopy`. Note that in contrast to
    /// [`bytemuck`], there is no major version marker here. This is because `zerocopy` still has an unstable `0.x`
    /// version. In case it becomes stable, this reexport will be replaced with a reexport carrying `v1` in its
    /// name.
    #[cfg(feature = "zerocopy")]
    pub use zerocopy;
}

/// Macro for deriving [`wnf`](crate) traits from [`bytemuck`](bytemuck_v1) traits
///
/// Note that there cannot be a blanket implementation of [`wnf`](crate) traits such as [`AnyBitPattern`] from
/// [`bytemuck`](bytemuck_v1) traits such as [`bytemuck::AnyBitPattern`](bytemuck_v1::AnyBitPattern). As
/// [`bytemuck`](bytemuck_v1) is an optional dependency of [`wnf`](crate), such a blanket implementation would have to
/// be behind a feature gate just like any other [`bytemuck`](bytemuck_v1)-related functionality of [`wnf`](crate).
/// However, adding a blanket implementation is a breaking change and due to the way the Cargo feature resolver works,
/// enabling a feature must not introduce a breaking change.
///
/// This macro provides an alternative to a blanket implementation by requiring you to explicitly opt in to the
/// implementation.
///
/// If you have a type that implements [`bytemuck::AnyBitPattern`](bytemuck_v1::AnyBitPattern),
/// [`bytemuck::CheckedBitPattern`](bytemuck_v1::CheckedBitPattern) or [`bytemuck::NoUninit`](bytemuck_v1::NoUninit),
/// you can derive the corresponding [`wnf`](crate) traits as follows:
/// ```
/// # #[macro_use] extern crate wnf;
/// # extern crate bytemuck_v1 as bytemuck;
/// #
/// # fn main() {
/// use wnf::derive_from_bytemuck_v1;
///
/// #[derive(bytemuck::AnyBitPattern, Copy, Clone)]
/// #[repr(C)]
/// struct Foo(u8, u16);
///
/// derive_from_bytemuck_v1!(AnyBitPattern for Foo);
///
/// #[derive(bytemuck::CheckedBitPattern, Copy, Clone)]
/// #[repr(C)]
/// struct Bar(char);
///
/// derive_from_bytemuck_v1!(CheckedBitPattern for Bar);
///
/// #[derive(bytemuck::NoUninit, Copy, Clone)]
/// #[repr(C)]
/// struct Baz(bool);
///
/// derive_from_bytemuck_v1!(NoUninit for Baz);
/// # }
/// ```
///
/// Note that in case you already derive [`AnyBitPattern`], you cannot additionally derive [`CheckedBitPattern`] as
/// there is already a blanket implementation of [`CheckedBitPattern`] for any type implementing [`AnyBitPattern`].
#[cfg(feature = "bytemuck_v1")]
#[macro_export]
macro_rules! derive_from_bytemuck_v1 {
    (AnyBitPattern for $type:ty) => {
        const _: fn() = || {
            use $crate::__reexports::bytemuck::v1 as bytemuck_v1;

            const fn assert_impl_any_bit_pattern<T: ?Sized + bytemuck_v1::AnyBitPattern>() {}
            assert_impl_any_bit_pattern::<$type>();

            // SAFETY:
            // - the above asserts that $type : bytemuck_v1::AnyBitPattern
            // - this implies the safety conditions of wnf::AnyBitPattern
            unsafe impl $crate::AnyBitPattern for $type {}
        };
    };

    (CheckedBitPattern for $type:ty) => {
        const _: fn() = || {
            use $crate::__reexports::bytemuck::v1 as bytemuck_v1;

            const fn assert_impl_checked_bit_pattern<T: ?Sized + bytemuck_v1::CheckedBitPattern>() {}
            assert_impl_checked_bit_pattern::<$type>();

            #[derive(Clone, Copy)]
            #[repr(transparent)]
            struct __wnf_derive_from_bytemuck_v1_Bits(<$type as bytemuck_v1::CheckedBitPattern>::Bits);

            // SAFETY:
            // - the inner type implements bytemuck_v1::AnyBitPattern
            // - this implies the safety conditions of wnf::AnyBitPattern
            // - this type is a #[repr(transparent)] wrapper around the inner type
            unsafe impl $crate::AnyBitPattern for __wnf_derive_from_bytemuck_v1_Bits {}

            // SAFETY:
            // - the implementation just delegates to bytemuck_v1::CheckedBitPattern
            // - this implies the safety conditions of wnf::CheckedBitPattern
            unsafe impl $crate::CheckedBitPattern for $type {
                type Bits = __wnf_derive_from_bytemuck_v1_Bits;

                fn is_valid_bit_pattern(bits: &Self::Bits) -> bool {
                    <$type as bytemuck_v1::CheckedBitPattern>::is_valid_bit_pattern(&bits.0)
                }
            }
        };
    };

    (NoUninit for $type:ty) => {
        const _: fn() = || {
            use $crate::__reexports::bytemuck::v1 as bytemuck_v1;

            const fn assert_impl_no_uninit<T: ?Sized + bytemuck_v1::NoUninit>() {}
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

/// Macro for deriving [`wnf`](crate) traits from [`zerocopy`] traits
///
/// Note that there cannot be a blanket implementation of [`wnf`](crate) traits such as [`AnyBitPattern`] from
/// [`zerocopy`] traits such as [`zerocopy::FromBytes`]. As [`zerocopy`] is an optional dependency of [`wnf`](crate),
/// such a blanket implementation would have to be behind a feature gate just like any other [`zerocopy`]-related
/// functionality of [`wnf`](crate). However, adding a blanket implementation is a breaking change and due to the way
/// the Cargo feature resolver works, enabling a feature must not introduce a breaking change.
///
/// This macro provides an alternative to a blanket implementation by requiring you to explicitly opt in to the
/// implementation.
///
/// If you have a type that implements [`zerocopy::FromBytes`] or [`zerocopy::AsBytes`], you can derive the
/// corresponding [`wnf`](crate) traits as follows: 
/// ```
/// # #[macro_use] extern crate wnf;
/// #
/// # fn main() {
/// use wnf::derive_from_zerocopy;
///
/// #[derive(zerocopy::FromBytes, Copy, Clone)]
/// #[repr(C)]
/// struct Foo(u8, u16);
///
/// derive_from_zerocopy!(AnyBitPattern for Foo);
///
/// #[derive(zerocopy::AsBytes, Copy, Clone)]
/// #[repr(C)]
/// struct Bar(bool);
///
/// derive_from_zerocopy!(NoUninit for Bar);
/// # }
/// ```
#[cfg(feature = "zerocopy")]
#[macro_export]
macro_rules! derive_from_zerocopy {
    (AnyBitPattern for $type:ty) => {
        const _: fn() = || {
            use $crate::__reexports::zerocopy;

            const fn assert_impl_from_bytes<T: ?Sized + zerocopy::FromBytes>() {}
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
            use $crate::__reexports::zerocopy;

            const fn assert_impl_as_bytes<T: ?Sized + zerocopy::AsBytes>() {}
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
