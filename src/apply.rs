use std::borrow::Borrow;
use std::convert::Infallible;
use std::error::Error;
use std::io;
use std::io::ErrorKind;

use crate::bytes::NoUninit;
use crate::read::WnfRead;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};

impl<T> OwnedWnfState<T>
where
    T: WnfRead<T> + NoUninit,
{
    pub fn apply<D, F>(&self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        F: FnMut(T) -> D,
    {
        self.raw.apply(transform)
    }

    pub fn try_apply<D, E, F>(&self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(T) -> Result<D, E>,
    {
        self.raw.try_apply(transform)
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfRead<Box<T>> + NoUninit + ?Sized,
{
    pub fn apply_boxed<D, F>(&self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> D,
    {
        self.raw.apply_boxed(transform)
    }

    pub fn try_apply_boxed<D, E, F>(&self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(Box<T>) -> Result<D, E>,
    {
        self.raw.try_apply_boxed(transform)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead<T> + NoUninit,
{
    pub fn apply<D, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        F: FnMut(T) -> D,
    {
        self.raw.apply(transform)
    }

    pub fn try_apply<D, E, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(T) -> Result<D, E>,
    {
        self.raw.try_apply(transform)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead<Box<T>> + NoUninit + ?Sized,
{
    pub fn apply_boxed<D, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> D,
    {
        self.raw.apply_boxed(transform)
    }

    pub fn try_apply_boxed<D, E, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(Box<T>) -> Result<D, E>,
    {
        self.raw.try_apply_boxed(transform)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<T> + NoUninit,
{
    pub fn apply<D, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        F: FnMut(T) -> D,
    {
        self.apply_as(transform)
    }

    pub fn try_apply<D, E, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(T) -> Result<D, E>,
    {
        self.try_apply_as(transform)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<Box<T>> + NoUninit + ?Sized,
{
    pub fn apply_boxed<D, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> D,
    {
        self.apply_as(transform)
    }

    pub fn try_apply_boxed<D, E, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(Box<T>) -> Result<D, E>,
    {
        self.try_apply_as(transform)
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    pub(crate) fn apply_as<ReadInto, WriteFrom, F>(self, mut transform: F) -> io::Result<WriteFrom>
    where
        WriteFrom: Borrow<T>,
        T: WnfRead<ReadInto> + NoUninit,
        F: FnMut(ReadInto) -> WriteFrom,
    {
        self.try_apply_as(|data| Ok::<_, Infallible>(transform(data)))
    }

    fn try_apply_as<ReadInto, WriteFrom, E, F>(self, mut transform: F) -> io::Result<WriteFrom>
    where
        WriteFrom: Borrow<T>,
        T: WnfRead<ReadInto> + NoUninit,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(ReadInto) -> Result<WriteFrom, E>,
    {
        let result = loop {
            let (data, change_stamp) = self.query_as()?.into_data_change_stamp();
            let result = transform(data).map_err(|err| io::Error::new(ErrorKind::Other, err))?;
            if self.update(result.borrow(), change_stamp)? {
                break result;
            }
        };

        Ok(result)
    }
}
