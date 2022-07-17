use std::borrow::Borrow;
use std::io;

use crate::state::RawWnfState;
use crate::{BorrowedWnfState, NoUninit, OwnedWnfState, WnfRead};

impl<T> OwnedWnfState<T>
where
    T: WnfRead<T> + NoUninit,
{
    pub fn replace<D>(&self, new_value: D) -> io::Result<T>
    where
        D: Borrow<T>,
    {
        self.raw.replace(new_value)
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfRead<Box<T>> + NoUninit + ?Sized,
{
    pub fn replace_boxed<D>(&self, new_value: D) -> io::Result<Box<T>>
    where
        D: Borrow<T>,
    {
        self.raw.replace_boxed(new_value)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead<T> + NoUninit,
{
    pub fn replace<D>(&self, new_value: D) -> io::Result<T>
    where
        D: Borrow<T>,
    {
        self.raw.replace(new_value)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead<Box<T>> + NoUninit + ?Sized,
{
    pub fn replace_boxed<D>(&self, new_value: D) -> io::Result<Box<T>>
    where
        D: Borrow<T>,
    {
        self.raw.replace_boxed(new_value)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<T> + NoUninit,
{
    pub fn replace<D>(&self, new_value: D) -> io::Result<T>
    where
        D: Borrow<T>,
    {
        self.replace_as(new_value)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<Box<T>> + NoUninit + ?Sized,
{
    pub fn replace_boxed<D>(&self, new_value: D) -> io::Result<Box<T>>
    where
        D: Borrow<T>,
    {
        self.replace_as(new_value)
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    fn replace_as<ReadInto, WriteFrom>(&self, new_value: WriteFrom) -> io::Result<ReadInto>
    where
        WriteFrom: Borrow<T>,
        T: WnfRead<ReadInto> + NoUninit,
    {
        let mut old_value = None;
        self.apply_as(|value| {
            old_value = Some(value);
            new_value.borrow()
        })?;
        Ok(old_value.unwrap())
    }
}
