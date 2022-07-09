use std::borrow::Borrow;
use std::convert::Infallible;

use thiserror::Error;

use crate::bytes::NoUninit;
use crate::query::WnfQueryError;
use crate::read::WnfRead;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::update::WnfUpdateError;
use crate::WnfChangeStamp;

impl<T> OwnedWnfState<T>
where
    T: WnfRead<T> + NoUninit,
{
    pub fn apply<D, F>(&self, transform: F) -> Result<D, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(T, WnfChangeStamp) -> D,
    {
        self.raw.apply(transform)
    }

    pub fn try_apply<D, E, F>(&self, transform: F) -> Result<D, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(T, WnfChangeStamp) -> Result<D, E>,
    {
        self.raw.try_apply(transform)
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfRead<Box<T>> + NoUninit + ?Sized,
{
    pub fn apply_boxed<D, F>(&self, transform: F) -> Result<D, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>, WnfChangeStamp) -> D,
    {
        self.raw.apply_boxed(transform)
    }

    pub fn try_apply_boxed<D, E, F>(&self, transform: F) -> Result<D, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>, WnfChangeStamp) -> Result<D, E>,
    {
        self.raw.try_apply_boxed(transform)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead<T> + NoUninit,
{
    pub fn apply<D, F>(&self, transform: F) -> Result<D, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(T, WnfChangeStamp) -> D,
    {
        self.raw.apply(transform)
    }

    pub fn try_apply<D, E, F>(&self, transform: F) -> Result<D, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(T, WnfChangeStamp) -> Result<D, E>,
    {
        self.raw.try_apply(transform)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead<Box<T>> + NoUninit + ?Sized,
{
    pub fn apply_boxed<D, F>(&self, transform: F) -> Result<D, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>, WnfChangeStamp) -> D,
    {
        self.raw.apply_boxed(transform)
    }

    pub fn try_apply_boxed<D, E, F>(&self, transform: F) -> Result<D, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>, WnfChangeStamp) -> Result<D, E>,
    {
        self.raw.try_apply_boxed(transform)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<T> + NoUninit,
{
    pub fn apply<D, F>(&self, mut transform: F) -> Result<D, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(T, WnfChangeStamp) -> D,
    {
        let result = loop {
            let (data, change_stamp) = self.query()?.into_data_change_stamp();
            let result = transform(data, change_stamp);
            if self.update(result.borrow(), change_stamp)? {
                break result;
            }
        };

        Ok(result)
    }

    pub fn try_apply<D, E, F>(&self, mut transform: F) -> Result<D, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(T, WnfChangeStamp) -> Result<D, E>,
    {
        let result = loop {
            let (data, change_stamp) = self.query()?.into_data_change_stamp();
            let result = transform(data, change_stamp).map_err(WnfTransformError::from)?;
            if self.update(result.borrow(), change_stamp)? {
                break result;
            }
        };

        Ok(result)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<Box<T>> + NoUninit + ?Sized,
{
    pub fn apply_boxed<D, F>(&self, mut transform: F) -> Result<D, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>, WnfChangeStamp) -> D,
    {
        let result = loop {
            let (data, change_stamp) = self.query_boxed()?.into_data_change_stamp();
            let result = transform(data, change_stamp);
            if self.update(result.borrow(), change_stamp)? {
                break result;
            }
        };

        Ok(result)
    }

    pub fn try_apply_boxed<D, E, F>(&self, mut transform: F) -> Result<D, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>, WnfChangeStamp) -> Result<D, E>,
    {
        let result = loop {
            let (data, change_stamp) = self.query_boxed()?.into_data_change_stamp();
            let result = transform(data, change_stamp).map_err(WnfTransformError::from)?;
            if self.update(result.borrow(), change_stamp)? {
                break result;
            }
        };

        Ok(result)
    }
}

#[derive(Debug, Error, PartialEq)]
#[error(transparent)]
pub struct WnfTransformError<E>(#[from] pub E);

#[derive(Debug, Error, PartialEq)]
pub enum WnfApplyError<E = Infallible> {
    #[error("failed to apply transformation to WNF state data: {0}")]
    Query(#[from] WnfQueryError),

    #[error("failed to apply transformation to WNF state data: failed to transform data: {0}")]
    Transform(#[from] WnfTransformError<E>),

    #[error("failed to apply transformation to WNF state data: {0}")]
    Update(#[from] WnfUpdateError),
}
