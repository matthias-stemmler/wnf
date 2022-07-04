use std::sync::{Arc, Condvar, Mutex};

use thiserror::Error;

use crate::read::WnfReadBoxed;
use crate::state::RawWnfState;
use crate::{
    BorrowedWnfState, OwnedWnfState, WnfDataAccessor, WnfQueryError, WnfRead, WnfReadError, WnfSubscribeError,
    WnfUnsubscribeError,
};

impl<T> OwnedWnfState<T> {
    pub fn wait_blocking(&self) -> Result<(), WnfWaitError> {
        self.raw.wait_blocking()
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfRead,
{
    pub fn wait_until_blocking<F>(&self, predicate: F) -> Result<T, WnfWaitError>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_blocking(predicate)
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfReadBoxed + ?Sized,
{
    pub fn wait_until_boxed_blocking<F>(&self, predicate: F) -> Result<Box<T>, WnfWaitError>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_boxed_blocking(predicate)
    }
}

impl<T> BorrowedWnfState<'_, T> {
    pub fn wait_blocking(&self) -> Result<(), WnfWaitError> {
        self.raw.wait_blocking()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead,
{
    pub fn wait_until_blocking<F>(&self, predicate: F) -> Result<T, WnfWaitError>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_blocking(predicate)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfReadBoxed + ?Sized,
{
    pub fn wait_until_boxed_blocking<F>(&self, predicate: F) -> Result<Box<T>, WnfWaitError>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_boxed_blocking(predicate)
    }
}

impl<T> RawWnfState<T> {
    pub fn wait_blocking(&self) -> Result<(), WnfWaitError> {
        let change_stamp = self.change_stamp()?;

        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let pair2 = Arc::clone(&pair);

        let subscription = self.subscribe(
            change_stamp,
            Box::new(move |_: &WnfDataAccessor<_>, _| {
                let (mutex, condvar) = &*pair2;
                *mutex.lock().unwrap() = true;
                condvar.notify_one();
            }),
        )?;

        let (mutex, condvar) = &*pair;
        drop(condvar.wait_while(mutex.lock().unwrap(), |updated| !*updated).unwrap());

        subscription.unsubscribe().map_err(|(err, _)| err)?;

        Ok(())
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead,
{
    pub fn wait_until_blocking<F>(&self, mut predicate: F) -> Result<T, WnfWaitError>
    where
        F: FnMut(&T) -> bool,
    {
        let (data, change_stamp) = self.query()?.into_data_change_stamp();

        if predicate(&data) {
            return Ok(data);
        }

        let pair = Arc::new((Mutex::new(Some(Ok(data))), Condvar::new()));
        let pair2 = Arc::clone(&pair);

        let subscription = self.subscribe(
            change_stamp,
            Box::new(move |accessor: &WnfDataAccessor<_>, _| {
                let (mutex, condvar) = &*pair2;
                *mutex.lock().unwrap() = Some(accessor.get());
                condvar.notify_one();
            }),
        )?;

        let (mutex, condvar) = &*pair;
        let mut guard = condvar
            .wait_while(
                mutex.lock().unwrap(),
                |result| matches!(result.as_ref().unwrap(), Ok(data) if !predicate(data)),
            )
            .unwrap();

        subscription.unsubscribe().map_err(|(err, _)| err)?;

        Ok(guard.take().unwrap()?)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfReadBoxed + ?Sized,
{
    pub fn wait_until_boxed_blocking<F>(&self, mut predicate: F) -> Result<Box<T>, WnfWaitError>
    where
        F: FnMut(&T) -> bool,
    {
        let (data, change_stamp) = self.query_boxed()?.into_data_change_stamp();

        if predicate(&data) {
            return Ok(data);
        }

        let pair = Arc::new((Mutex::new(Some(Ok(data))), Condvar::new()));
        let pair2 = Arc::clone(&pair);

        let subscription = self.subscribe(
            change_stamp,
            Box::new(move |accessor: &WnfDataAccessor<_>, _| {
                let (mutex, condvar) = &*pair2;
                *mutex.lock().unwrap() = Some(accessor.get_boxed());
                condvar.notify_one();
            }),
        )?;

        let (mutex, condvar) = &*pair;
        let mut guard = condvar
            .wait_while(
                mutex.lock().unwrap(),
                |result| matches!(result.as_ref().unwrap(), Ok(data) if !predicate(data)),
            )
            .unwrap();

        subscription.unsubscribe().map_err(|(err, _)| err)?;

        Ok(guard.take().unwrap()?)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfWaitError {
    #[error("failed to wait for WNF state update: {0}")]
    Query(#[from] WnfQueryError),

    #[error("failed to wait for WNF state update: {0}")]
    Read(#[from] WnfReadError),

    #[error("failed to wait for WNF state update: {0}")]
    Subscribe(#[from] WnfSubscribeError),

    #[error("failed to wait for WNF state update: {0}")]
    Unsubscribe(#[from] WnfUnsubscribeError),
}
