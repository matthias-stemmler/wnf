use std::borrow::Borrow;
use std::convert::Infallible;
use std::ops::ControlFlow;

use thiserror::Error;

use crate::bytes::NoUninit;
use crate::callback::WnfCallback;
use crate::query::WnfQueryError;
use crate::read::{WnfRead, WnfReadBoxed};
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::update::WnfUpdateError;

pub trait WnfTransformResult<T> {
    fn data(&self) -> ControlFlow<(), &T>;
}

impl<T> WnfTransformResult<T> for T {
    fn data(&self) -> ControlFlow<(), &T> {
        ControlFlow::Continue(self)
    }
}

impl<T> WnfTransformResult<T> for ControlFlow<(), T> {
    fn data(&self) -> ControlFlow<(), &T> {
        match *self {
            ControlFlow::Continue(ref data) => ControlFlow::Continue(data),
            ControlFlow::Break(()) => ControlFlow::Break(()),
        }
    }
}

impl<T, Meta> WnfTransformResult<T> for (T, Meta) {
    fn data(&self) -> ControlFlow<(), &T> {
        ControlFlow::Continue(&self.0)
    }
}

impl<T, Meta> WnfTransformResult<T> for (ControlFlow<(), T>, Meta) {
    fn data(&self) -> ControlFlow<(), &T> {
        match self.0 {
            ControlFlow::Continue(ref data) => ControlFlow::Continue(data),
            ControlFlow::Break(()) => ControlFlow::Break(()),
        }
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfRead + NoUninit,
{
    pub fn apply<D, F, Args, Return>(&self, transform: F) -> Result<Return, WnfApplyError>
    where
        D: Borrow<T>,
        F: WnfCallback<T, Args, Return>,
        Return: WnfTransformResult<D>,
    {
        self.raw.apply(transform)
    }

    pub fn try_apply<D, E, F, Args, Return>(&self, transform: F) -> Result<Return, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: WnfCallback<T, Args, Result<Return, E>>,
        Return: WnfTransformResult<D>,
    {
        self.raw.try_apply(transform)
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfReadBoxed + NoUninit + ?Sized,
{
    pub fn apply_boxed<D, F, Args, Return>(&self, transform: F) -> Result<Return, WnfApplyError>
    where
        D: Borrow<T>,
        F: WnfCallback<Box<T>, Args, Return>,
        Return: WnfTransformResult<D>,
    {
        self.raw.apply_boxed(transform)
    }

    pub fn try_apply_boxed<D, E, F, Args, Return>(&self, transform: F) -> Result<Return, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: WnfCallback<Box<T>, Args, Result<Return, E>>,
        Return: WnfTransformResult<D>,
    {
        self.raw.try_apply_boxed(transform)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead + NoUninit,
{
    pub fn apply<D, F, Args, Return>(&self, transform: F) -> Result<Return, WnfApplyError>
    where
        D: Borrow<T>,
        F: WnfCallback<T, Args, Return>,
        Return: WnfTransformResult<D>,
    {
        self.raw.apply(transform)
    }

    pub fn try_apply<D, E, F, Args, Return>(&self, transform: F) -> Result<Return, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: WnfCallback<T, Args, Result<Return, E>>,
        Return: WnfTransformResult<D>,
    {
        self.raw.try_apply(transform)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfReadBoxed + NoUninit + ?Sized,
{
    pub fn apply_boxed<D, F, Args, Return>(&self, transform: F) -> Result<Return, WnfApplyError>
    where
        D: Borrow<T>,
        F: WnfCallback<Box<T>, Args, Return>,
        Return: WnfTransformResult<D>,
    {
        self.raw.apply_boxed(transform)
    }

    pub fn try_apply_boxed<D, E, F, Args, Return>(&self, transform: F) -> Result<Return, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: WnfCallback<Box<T>, Args, Result<Return, E>>,
        Return: WnfTransformResult<D>,
    {
        self.raw.try_apply_boxed(transform)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead + NoUninit,
{
    pub fn apply<D, F, Args, Return>(&self, mut transform: F) -> Result<Return, WnfApplyError>
    where
        D: Borrow<T>,
        F: WnfCallback<T, Args, Return>,
        Return: WnfTransformResult<D>,
    {
        let result = loop {
            let (data, change_stamp) = self.query()?.into_data_change_stamp();
            let result = transform.call(data, change_stamp);
            match result.data() {
                ControlFlow::Break(()) => break result,
                ControlFlow::Continue(data) => {
                    if self.update(data.borrow(), change_stamp)? {
                        break result;
                    }
                }
            }
        };

        Ok(result)
    }

    pub fn try_apply<D, E, F, Args, Return>(&self, mut transform: F) -> Result<Return, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: WnfCallback<T, Args, Result<Return, E>>,
        Return: WnfTransformResult<D>,
    {
        let result = loop {
            let (data, change_stamp) = self.query()?.into_data_change_stamp();
            let result = transform.call(data, change_stamp).map_err(WnfTransformError::from)?;
            match result.data() {
                ControlFlow::Break(()) => break result,
                ControlFlow::Continue(data) => {
                    if self.update(data.borrow(), change_stamp)? {
                        break result;
                    }
                }
            }
        };

        Ok(result)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfReadBoxed + NoUninit + ?Sized,
{
    pub fn apply_boxed<D, F, Args, Return>(&self, mut transform: F) -> Result<Return, WnfApplyError>
    where
        D: Borrow<T>,
        F: WnfCallback<Box<T>, Args, Return>,
        Return: WnfTransformResult<D>,
    {
        let result = loop {
            let (data, change_stamp) = self.query_boxed()?.into_data_change_stamp();
            let result = transform.call(data, change_stamp);
            match result.data() {
                ControlFlow::Break(()) => break result,
                ControlFlow::Continue(data) => {
                    if self.update(data.borrow(), change_stamp)? {
                        break result;
                    }
                }
            }
        };

        Ok(result)
    }

    pub fn try_apply_boxed<D, E, F, Args, Return>(&self, mut transform: F) -> Result<Return, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: WnfCallback<Box<T>, Args, Result<Return, E>>,
        Return: WnfTransformResult<D>,
    {
        let result = loop {
            let (data, change_stamp) = self.query_boxed()?.into_data_change_stamp();
            let result = transform.call(data, change_stamp).map_err(WnfTransformError::from)?;
            match result.data() {
                ControlFlow::Break(()) => break result,
                ControlFlow::Continue(data) => {
                    if self.update(data.borrow(), change_stamp)? {
                        break result;
                    }
                }
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
    #[error("failed to apply transformation to WNF state data: failed to query data: {0}")]
    Query(#[from] WnfQueryError),

    #[error("failed to apply transformation to WNF state data: failed to transform data: {0}")]
    Transform(#[from] WnfTransformError<E>),

    #[error("failed to apply transformation to WNF state data: failed to update data: {0}")]
    Update(#[from] WnfUpdateError),
}
