use crate::bytes::NoUninit;
use std::mem;

pub trait WnfWrite {
    fn write<F, R>(&self, write_raw: F) -> R
    where
        F: FnMut(*const Self, usize) -> R;
}

impl<T> WnfWrite for T
where
    T: NoUninit,
{
    fn write<F, R>(&self, mut write_raw: F) -> R
    where
        F: FnMut(*const Self, usize) -> R,
    {
        write_raw(self, mem::size_of::<Self>())
    }
}

impl<T> WnfWrite for [T]
where
    T: NoUninit,
{
    fn write<F, R>(&self, mut write_raw: F) -> R
    where
        F: FnMut(*const Self, usize) -> R,
    {
        write_raw(self, self.len() * mem::size_of::<T>())
    }
}
