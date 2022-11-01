#![deny(unsafe_code)]

use std::borrow::Borrow;
use std::io;
use std::sync::{Arc, Condvar, Mutex};

use crate::data::OpaqueData;
use crate::predicate::{ChangedPredicate, Predicate, PredicateStage};
use crate::read::Read;
use crate::state::{BorrowedState, OwnedState, RawState};
use crate::subscribe::{DataAccessor, SeenChangeStamp};

impl<T> OwnedState<T>
where
    T: ?Sized,
{
    pub fn wait_blocking(&self) -> io::Result<()> {
        self.raw.wait_blocking()
    }
}

impl<T> OwnedState<T>
where
    T: Read<T>,
{
    pub fn wait_until_blocking<F>(&self, predicate: F) -> io::Result<T>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_blocking(predicate)
    }
}

impl<T> OwnedState<T>
where
    T: Read<Box<T>> + ?Sized,
{
    pub fn wait_until_boxed_blocking<F>(&self, predicate: F) -> io::Result<Box<T>>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_boxed_blocking(predicate)
    }
}

impl<T> BorrowedState<'_, T>
where
    T: ?Sized,
{
    pub fn wait_blocking(self) -> io::Result<()> {
        self.raw.wait_blocking()
    }
}

impl<T> BorrowedState<'_, T>
where
    T: Read<T>,
{
    pub fn wait_until_blocking<F>(self, predicate: F) -> io::Result<T>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_blocking(predicate)
    }
}

impl<T> BorrowedState<'_, T>
where
    T: Read<Box<T>> + ?Sized,
{
    pub fn wait_until_boxed_blocking<F>(self, predicate: F) -> io::Result<Box<T>>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_boxed_blocking(predicate)
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    fn wait_blocking(self) -> io::Result<()> {
        let _: OpaqueData = self.cast().wait_until_blocking_internal(ChangedPredicate)?;
        Ok(())
    }
}

impl<T> RawState<T>
where
    T: Read<T>,
{
    fn wait_until_blocking<F>(self, predicate: F) -> io::Result<T>
    where
        F: FnMut(&T) -> bool,
    {
        self.wait_until_blocking_internal(predicate)
    }
}

impl<T> RawState<T>
where
    T: Read<Box<T>> + ?Sized,
{
    fn wait_until_boxed_blocking<F>(self, predicate: F) -> io::Result<Box<T>>
    where
        F: FnMut(&T) -> bool,
    {
        self.wait_until_blocking_internal(predicate)
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    fn wait_until_blocking_internal<D, F>(self, mut predicate: F) -> io::Result<D>
    where
        D: Borrow<T> + Send + 'static,
        F: Predicate<T>,
        T: Read<D>,
    {
        let (data, change_stamp) = self.query_as()?.into_data_change_stamp();

        if predicate.check(data.borrow(), PredicateStage::Initial) {
            return Ok(data);
        }

        let pair = Arc::new((Mutex::new(Some(Ok(data))), Condvar::new()));
        let pair_for_subscription = Arc::clone(&pair);

        let subscription = self.subscribe(
            move |accessor: DataAccessor<_>| {
                let (mutex, condvar) = &*pair_for_subscription;
                *mutex.lock().unwrap() = Some(accessor.get_as());
                condvar.notify_one();
            },
            SeenChangeStamp::Value(change_stamp),
        )?;

        let (mutex, condvar) = &*pair;
        let mut guard = condvar
            .wait_while(
                mutex.lock().unwrap(),
                |result| matches!(result.as_ref().unwrap(), Ok(data) if !predicate.check(data.borrow(), PredicateStage::Changed)),
            )
            .unwrap();

        subscription.unsubscribe()?;

        guard.take().unwrap()
    }
}
