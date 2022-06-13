// must hold: size is multiple of alignment
/// # Safety
/// TODO
pub unsafe trait AnyBitPattern: Copy + Send + Sized + 'static {}

/// # Safety
/// TODO
pub unsafe trait CheckedBitPattern: Copy + Send + Sized + 'static {
    type Bits: AnyBitPattern;

    fn is_valid_bit_pattern(bits: &Self::Bits) -> bool;
}

/// # Safety
/// TODO
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

// must hold: size is multiple of alignment
/// # Safety
/// TODO
pub unsafe trait NoUninit {}

unsafe impl AnyBitPattern for () {}
unsafe impl AnyBitPattern for u8 {}
unsafe impl AnyBitPattern for u16 {}
unsafe impl AnyBitPattern for u32 {}
unsafe impl AnyBitPattern for u64 {}

unsafe impl<T, const N: usize> AnyBitPattern for [T; N] where T: AnyBitPattern {}

unsafe impl NoUninit for () {}
unsafe impl NoUninit for u8 {}
unsafe impl NoUninit for u16 {}
unsafe impl NoUninit for u32 {}
unsafe impl NoUninit for u64 {}

unsafe impl<T, const N: usize> NoUninit for [T; N] where T: NoUninit {}
unsafe impl<T> NoUninit for [T] where T: NoUninit {}
