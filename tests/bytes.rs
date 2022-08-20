#[cfg(feature = "bytemuck_v1")]
mod bytemuck_v1_tests {
    use wnf::{derive_from_bytemuck_v1, AnyBitPattern, CheckedBitPattern, NoUninit};

    #[test]
    fn derive_any_bit_pattern_from_bytemuck() {
        #[derive(Copy, Clone)]
        struct Test;

        // SAFETY: `Test` is zero-sized
        unsafe impl bytemuck_v1::Zeroable for Test {}
        unsafe impl bytemuck_v1::AnyBitPattern for Test {}

        derive_from_bytemuck_v1!(AnyBitPattern for Test);

        assert_impl_any_bit_pattern::<Test>();
    }

    #[test]
    fn derive_checked_bit_pattern_from_bytemuck() {
        #[derive(Copy, Clone)]
        struct Test;

        // SAFETY: `Test` is zero-sized
        unsafe impl bytemuck_v1::CheckedBitPattern for Test {
            type Bits = u8;

            fn is_valid_bit_pattern(bits: &Self::Bits) -> bool {
                *bits < 128
            }
        }

        derive_from_bytemuck_v1!(CheckedBitPattern for Test);

        assert!(<Test as CheckedBitPattern>::is_valid_bit_pattern(&127u8));
        assert!(!<Test as CheckedBitPattern>::is_valid_bit_pattern(&128u8));
    }

    #[test]
    fn derive_no_uninit_from_bytemuck() {
        #[derive(Copy, Clone)]
        struct Test;

        // SAFETY: `Test` is zero-sized
        unsafe impl bytemuck_v1::NoUninit for Test {}

        derive_from_bytemuck_v1!(NoUninit for Test);

        assert_impl_no_uninit::<Test>();
    }

    fn assert_impl_any_bit_pattern<T: AnyBitPattern>() {}
    fn assert_impl_no_uninit<T: NoUninit>() {}
}

#[cfg(feature = "zerocopy")]
mod zerocopy_tests {
    use wnf::{derive_from_zerocopy, AnyBitPattern, NoUninit};

    #[test]
    fn derive_any_bit_pattern_from_zerocopy() {
        #[derive(Copy, Clone, zerocopy::FromBytes)]
        struct Test;

        derive_from_zerocopy!(AnyBitPattern for Test);

        assert_impl_any_bit_pattern::<Test>();
    }

    #[test]
    fn derive_no_uninit_from_zerocopy() {
        #[derive(Copy, Clone, zerocopy::AsBytes)]
        #[repr(C)]
        struct Test;

        derive_from_zerocopy!(NoUninit for Test);

        assert_impl_no_uninit::<Test>();
    }

    fn assert_impl_any_bit_pattern<T: AnyBitPattern>() {}
    fn assert_impl_no_uninit<T: NoUninit>() {}
}
