pub unsafe trait AnyBitPattern: Copy + Sized + 'static {}

pub unsafe trait CheckedBitPattern: Copy + Sized + 'static {
    type Bits: AnyBitPattern;

    fn is_valid_bit_pattern(bits: &Self::Bits) -> bool;
}

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

pub unsafe trait NoUninit: Copy + Sized + 'static {}

unsafe impl AnyBitPattern for u32 {}
unsafe impl AnyBitPattern for () {}

unsafe impl NoUninit for u32 {}
unsafe impl NoUninit for () {}

unsafe impl<T, const N: usize> AnyBitPattern for [T; N] where T: AnyBitPattern {}
