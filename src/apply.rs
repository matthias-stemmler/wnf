use std::borrow::Borrow;
use std::convert::Infallible;

use thiserror::Error;

use crate::bytes::{CheckedBitPattern, NoUninit};
use crate::query::WnfQueryError;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::update::WnfUpdateError;

impl<T> OwnedWnfState<T>
where
    T: CheckedBitPattern + NoUninit,
{
    pub fn apply<D, F>(&self, transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(T) -> Option<D>,
    {
        self.raw.apply(transform)
    }

    pub fn apply_boxed<D, F>(&self, transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Option<D>,
    {
        self.raw.apply_boxed(transform)
    }

    pub fn apply_slice<D, F>(&self, transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Option<D>,
    {
        self.raw.apply_slice(transform)
    }

    pub fn try_apply<D, E, F>(&self, tranform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(T) -> Result<Option<D>, E>,
    {
        self.raw.try_apply(tranform)
    }

    pub fn try_apply_boxed<D, E, F>(&self, transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Result<Option<D>, E>,
    {
        self.raw.try_apply_boxed(transform)
    }

    pub fn try_apply_slice<D, E, F>(&self, transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Result<Option<D>, E>,
    {
        self.raw.try_apply_slice(transform)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: CheckedBitPattern + NoUninit,
{
    pub fn apply<D, F>(&self, transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(T) -> Option<D>,
    {
        self.raw.apply(transform)
    }

    pub fn apply_boxed<D, F>(&self, transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Option<D>,
    {
        self.raw.apply_boxed(transform)
    }

    pub fn apply_slice<D, F>(&self, transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Option<D>,
    {
        self.raw.apply_slice(transform)
    }

    pub fn try_apply<D, E, F>(&self, transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(T) -> Result<Option<D>, E>,
    {
        self.raw.try_apply(transform)
    }

    pub fn try_apply_boxed<D, E, F>(&self, transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Result<Option<D>, E>,
    {
        self.raw.try_apply_boxed(transform)
    }

    pub fn try_apply_slice<D, E, F>(&self, transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Result<Option<D>, E>,
    {
        self.raw.try_apply_slice(transform)
    }
}

impl<T> RawWnfState<T>
where
    T: CheckedBitPattern + NoUninit,
{
    pub fn apply<D, F>(&self, mut transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(T) -> Option<D>,
    {
        loop {
            let (data, change_stamp) = self.query()?.into_data_change_stamp();
            match transform(data) {
                None => return Ok(false),
                Some(data) => {
                    if self.update(data, change_stamp)? {
                        return Ok(true);
                    }
                }
            }
        }
    }

    pub fn apply_boxed<D, F>(&self, mut transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Option<D>,
    {
        loop {
            let (data, change_stamp) = self.query_boxed()?.into_data_change_stamp();
            match transform(data) {
                None => return Ok(false),
                Some(data) => {
                    if self.update(data, change_stamp)? {
                        return Ok(true);
                    }
                }
            }
        }
    }

    pub fn apply_slice<D, F>(&self, mut transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Option<D>,
    {
        loop {
            let (data, change_stamp) = self.query_slice()?.into_data_change_stamp();
            match transform(data) {
                None => return Ok(false),
                Some(data) => {
                    if self.update_slice(data, change_stamp)? {
                        return Ok(true);
                    }
                }
            }
        }
    }

    pub fn try_apply<D, E, F>(&self, mut transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(T) -> Result<Option<D>, E>,
    {
        loop {
            let (data, change_stamp) = self.query()?.into_data_change_stamp();
            match transform(data).map_err(WnfTransformError::from)? {
                None => return Ok(false),
                Some(data) => {
                    if self.update(data, change_stamp)? {
                        return Ok(true);
                    }
                }
            }
        }
    }

    pub fn try_apply_boxed<D, E, F>(&self, mut transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Result<Option<D>, E>,
    {
        loop {
            let (data, change_stamp) = self.query_boxed()?.into_data_change_stamp();
            match transform(data).map_err(WnfTransformError::from)? {
                None => return Ok(false),
                Some(data) => {
                    if self.update(data, change_stamp)? {
                        return Ok(true);
                    }
                }
            }
        }
    }

    pub fn try_apply_slice<D, E, F>(&self, mut transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Result<Option<D>, E>,
    {
        loop {
            let (data, change_stamp) = self.query_slice()?.into_data_change_stamp();
            match transform(data).map_err(WnfTransformError::from)? {
                None => return Ok(false),
                Some(data) => {
                    if self.update_slice(data, change_stamp)? {
                        return Ok(true);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Error, PartialEq)]
#[error(transparent)]
pub struct WnfTransformError<E>(#[from] pub E);

#[derive(Debug, Error, PartialEq)]
pub enum WnfApplyError<E = Infallible> {
    #[error("failed to apply transformation to WNF state data: failed to query data: {0}")]
    Query(#[from] WnfQueryError),

    #[error("failed to apply transformation to WNF state data: failed to transform data: {0}")]
    Transform(#[from] WnfTransformError<E>),

    #[error("failed to apply transformation to WNF state data: failed to update data: {0}")]
    Update(#[from] WnfUpdateError),
}
