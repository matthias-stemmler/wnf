use std::borrow::Borrow;
use std::sync::{Arc, Condvar, Mutex};

use thiserror::Error;

use crate::state::RawWnfState;
use crate::{
    BorrowedWnfState, OwnedWnfState, WnfDataAccessor, WnfOpaqueData, WnfQueryError, WnfRead, WnfReadError,
    WnfSubscribeError, WnfUnsubscribeError,
};

impl<T> OwnedWnfState<T> {
    pub fn wait_blocking(&self) -> Result<(), WnfWaitError> {
        self.raw.wait_blocking()
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfRead<T>,
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
    T: WnfRead<Box<T>> + ?Sized,
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
    T: WnfRead<T>,
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
    T: WnfRead<Box<T>> + ?Sized,
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
        let _: WnfOpaqueData = self.cast().wait_until_blocking_internal(ChangedPredicate)?;
        Ok(())
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<T>,
{
    pub fn wait_until_blocking<F>(&self, predicate: F) -> Result<T, WnfWaitError>
    where
        F: FnMut(&T) -> bool,
    {
        self.wait_until_blocking_internal(predicate)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    pub fn wait_until_boxed_blocking<F>(&self, predicate: F) -> Result<Box<T>, WnfWaitError>
    where
        F: FnMut(&T) -> bool,
    {
        self.wait_until_blocking_internal(predicate)
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    fn wait_until_blocking_internal<D, F>(&self, mut predicate: F) -> Result<D, WnfWaitError>
    where
        D: Borrow<T> + Send + 'static,
        F: Predicate<T>,
        T: WnfRead<D>,
    {
        let (data, change_stamp) = self.query_as()?.into_data_change_stamp();

        if predicate.check(data.borrow(), PredicateStage::Initial) {
            return Ok(data);
        }

        let pair = Arc::new((Mutex::new(Some(Ok(data))), Condvar::new()));
        let pair2 = Arc::clone(&pair);

        let subscription = self.subscribe(
            change_stamp,
            Box::new(move |accessor: WnfDataAccessor<_>| {
                let (mutex, condvar) = &*pair2;
                *mutex.lock().unwrap() = Some(accessor.get_as());
                condvar.notify_one();
            }),
        )?;

        let (mutex, condvar) = &*pair;
        let mut guard = condvar
            .wait_while(
                mutex.lock().unwrap(),
                |result| matches!(result.as_ref().unwrap(), Ok(data) if !predicate.check(data.borrow(), PredicateStage::Changed)),
            )
            .unwrap();

        subscription.unsubscribe().map_err(|(err, _)| err)?;

        Ok(guard.take().unwrap()?)
    }
}

#[derive(Clone, Copy, Debug)]
enum PredicateStage {
    Initial,
    Changed,
}

trait Predicate<T>
where
    T: ?Sized,
{
    fn check(&mut self, data: &T, stage: PredicateStage) -> bool;
}

impl<F, T> Predicate<T> for F
where
    F: FnMut(&T) -> bool,
    T: ?Sized,
{
    fn check(&mut self, data: &T, _: PredicateStage) -> bool {
        self(data)
    }
}

struct ChangedPredicate;

impl<T> Predicate<T> for ChangedPredicate {
    fn check(&mut self, _: &T, stage: PredicateStage) -> bool {
        matches!(stage, PredicateStage::Changed)
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
