pub unsafe trait AnyBitPattern: Sized + Copy + 'static {}

pub unsafe trait CheckedBitPattern: Copy {
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

pub unsafe trait NoUninit: Sized + Copy + 'static {}

unsafe impl AnyBitPattern for u32 {}
unsafe impl NoUninit for u32 {}
