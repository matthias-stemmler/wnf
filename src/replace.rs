#![deny(unsafe_code)]

use std::io;

use crate::bytes::NoUninit;
use crate::read::Read;
use crate::state::{BorrowedState, OwnedState, RawState};

impl<T> OwnedState<T>
where
    T: Read<T> + NoUninit,
{
    pub fn replace(&self, new_value: &T) -> io::Result<T> {
        self.raw.replace(new_value)
    }
}

impl<T> OwnedState<T>
where
    T: Read<Box<T>> + NoUninit + ?Sized,
{
    pub fn replace_boxed(&self, new_value: &T) -> io::Result<Box<T>> {
        self.raw.replace_boxed(new_value)
    }
}

impl<T> BorrowedState<'_, T>
where
    T: Read<T> + NoUninit,
{
    pub fn replace(self, new_value: &T) -> io::Result<T> {
        self.raw.replace(new_value)
    }
}

impl<T> BorrowedState<'_, T>
where
    T: Read<Box<T>> + NoUninit + ?Sized,
{
    pub fn replace_boxed(self, new_value: &T) -> io::Result<Box<T>> {
        self.raw.replace_boxed(new_value)
    }
}

impl<T> RawState<T>
where
    T: Read<T> + NoUninit,
{
    fn replace(self, new_value: &T) -> io::Result<T> {
        self.replace_as(new_value)
    }
}

impl<T> RawState<T>
where
    T: Read<Box<T>> + NoUninit + ?Sized,
{
    fn replace_boxed(self, new_value: &T) -> io::Result<Box<T>> {
        self.replace_as(new_value)
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    fn replace_as<D>(self, new_value: &T) -> io::Result<D>
    where
        T: Read<D> + NoUninit,
    {
        let mut old_value = None;
        self.apply_as(|value| {
            old_value = Some(value);
            new_value
        })?;
        Ok(old_value.unwrap())
    }
}
