// #[derive(bytemuck::CheckedBitPattern)] uses #[cfg(target_arch = "spirv")]
#![allow(unexpected_cfgs)]

extern crate bytemuck_v1 as bytemuck;

use wnf::{derive_from_bytemuck_v1, derive_from_zerocopy, AnyBitPattern, CheckedBitPattern, NoUninit};

#[test]
fn derive_any_bit_pattern_from_bytemuck() {
    #[derive(bytemuck::AnyBitPattern, Clone, Copy)]
    #[repr(C)]
    struct Test(u8, u16);

    derive_from_bytemuck_v1!(AnyBitPattern for Test);

    assert_impl_any_bit_pattern::<Test>();
}

#[test]
fn derive_checked_bit_pattern_from_bytemuck() {
    #[derive(bytemuck::CheckedBitPattern, Clone, Copy)]
    #[repr(C)]
    struct Test(char);

    derive_from_bytemuck_v1!(CheckedBitPattern for Test);

    assert_impl_checked_bit_pattern::<Test>();
}

#[test]
fn derive_no_uninit_from_bytemuck() {
    #[derive(bytemuck::NoUninit, Clone, Copy)]
    #[repr(C)]
    struct Test(bool);

    derive_from_bytemuck_v1!(NoUninit for Test);

    assert_impl_no_uninit::<Test>();
}

#[test]
fn derive_any_bit_pattern_from_zerocopy() {
    #[derive(zerocopy_derive::FromBytes, Clone, Copy)]
    #[repr(C)]
    struct Test(u8, u16);

    derive_from_zerocopy!(AnyBitPattern for Test);

    assert_impl_any_bit_pattern::<Test>();
}

#[test]
fn derive_no_uninit_from_zerocopy() {
    #[derive(zerocopy_derive::IntoBytes, Clone, Copy)]
    #[repr(C)]
    struct Test(bool);

    derive_from_zerocopy!(NoUninit for Test);

    assert_impl_no_uninit::<Test>();
}

fn assert_impl_any_bit_pattern<T: AnyBitPattern>() {}
fn assert_impl_checked_bit_pattern<T: CheckedBitPattern>() {}
fn assert_impl_no_uninit<T: NoUninit>() {}
