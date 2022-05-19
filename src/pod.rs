use std::{
    marker::{PhantomData, PhantomPinned},
    mem::ManuallyDrop,
    num,
};

pub unsafe trait Pod: Sized + Copy + 'static {}

unsafe impl Pod for () {}

unsafe impl Pod for u8 {}
unsafe impl Pod for i8 {}
unsafe impl Pod for u16 {}
unsafe impl Pod for i16 {}
unsafe impl Pod for u32 {}
unsafe impl Pod for i32 {}
unsafe impl Pod for u64 {}
unsafe impl Pod for i64 {}
unsafe impl Pod for u128 {}
unsafe impl Pod for i128 {}
unsafe impl Pod for usize {}
unsafe impl Pod for isize {}

unsafe impl Pod for f32 {}
unsafe impl Pod for f64 {}

unsafe impl Pod for Option<num::NonZeroU8> {}
unsafe impl Pod for Option<num::NonZeroI8> {}
unsafe impl Pod for Option<num::NonZeroU16> {}
unsafe impl Pod for Option<num::NonZeroI16> {}
unsafe impl Pod for Option<num::NonZeroU32> {}
unsafe impl Pod for Option<num::NonZeroI32> {}
unsafe impl Pod for Option<num::NonZeroU64> {}
unsafe impl Pod for Option<num::NonZeroI64> {}
unsafe impl Pod for Option<num::NonZeroU128> {}
unsafe impl Pod for Option<num::NonZeroI128> {}
unsafe impl Pod for Option<num::NonZeroUsize> {}
unsafe impl Pod for Option<num::NonZeroIsize> {}

unsafe impl Pod for PhantomPinned {}

unsafe impl<T: Pod, const N: usize> Pod for [T; N] {}
unsafe impl<T: Pod> Pod for num::Wrapping<T> {}
unsafe impl<T: Pod> Pod for PhantomData<T> {}
unsafe impl<T: Pod> Pod for ManuallyDrop<T> {}
