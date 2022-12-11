//! Methods for synchronously waiting for state updates

#![deny(unsafe_code)]

use std::borrow::Borrow;
use std::io::{self, ErrorKind};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::data::OpaqueData;
use crate::predicate::{ChangedPredicate, Predicate, PredicateStage};
use crate::read::Read;
use crate::state::{BorrowedState, OwnedState, RawState};
use crate::subscribe::{DataAccessor, SeenChangeStamp};

impl<T> OwnedState<T>
where
    T: ?Sized,
{
    /// Waits until this state is updated
    ///
    /// This waits for *any* update to the state regardless of the value, even if the value is the same as the previous
    /// one. In order to wait until the state data satisfy a certain condition, use
    /// [`wait_until_blocking`](OwnedState::wait_until_blocking).
    ///
    /// Use this method if you want to wait for a state update *once*. In order to execute some logic on every state
    /// update, use the [`subscribe`](OwnedState::subscribe) method.
    ///
    /// This is a blocking method. If you are in an async context, use [`wait_async`](OwnedState::wait_async).
    ///
    /// # Errors
    /// Returns an error if querying, subscribing to or unsubscribing from the state fails or if the timeout has elapsed
    /// In the latter case, [`io::Error::kind`] returns [`ErrorKind::TimedOut`]
    pub fn wait_blocking(&self, timeout: Duration) -> io::Result<()> {
        self.raw.wait_blocking(timeout)
    }
}

impl<T> OwnedState<T>
where
    T: Read<T>,
{
    /// Waits until the data of this state satisfy a given predicate, returning the data
    ///
    /// This returns immediately if the current data already satisfy the predicate. Otherwise, it waits until the state
    /// is updated with data that satisfy the predicate. If you want to unconditionally wait until the state is updated,
    /// use [`wait_blocking`](OwnedState::wait_blocking).
    ///
    /// This returns the data for which the predicate returned `true`, causing the wait to finish. It produces an owned
    /// `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for `T: ?Sized`, use the
    /// [`wait_until_boxed_blocking`](OwnedState::wait_until_boxed_blocking) method.
    ///
    /// For example, to wait until the value of a state reaches a given minimum:
    /// ```
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// use std::{io, thread};
    ///
    /// use wnf::{AsState, OwnedState};
    ///
    /// fn wait_until_at_least<S>(state: S, min_value: u32) -> io::Result<u32>
    /// where
    ///     S: AsState<Data = u32>,
    /// {
    ///     state
    ///         .as_state()
    ///         .wait_until_blocking(|value| *value >= min_value, Duration::MAX)
    /// }
    ///
    /// let state = Arc::new(OwnedState::create_temporary().expect("failed to create state"));
    /// state.set(&0).expect("failed to set state data");
    ///
    /// {
    ///     let state = Arc::clone(&state);
    ///     thread::spawn(move || loop {
    ///         state.apply(|value| value + 1).unwrap();
    ///         thread::sleep(Duration::from_millis(10));
    ///     });
    /// }
    ///
    /// let value = wait_until_at_least(&state, 10).expect("failed to wait for state update");
    /// assert!(value >= 10);
    /// ```
    ///
    /// This is a blocking method. If you are in an async context, use
    /// [`wait_until_async`](OwnedState::wait_until_async).
    ///
    /// # Errors
    /// Returns an error if querying, subscribing to or unsubscribing from the state fails or if the timeout has elapsed
    /// In the latter case, [`io::Error::kind`] returns [`ErrorKind::TimedOut`]
    pub fn wait_until_blocking<F>(&self, predicate: F, timeout: Duration) -> io::Result<T>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_blocking(predicate, timeout)
    }
}

impl<T> OwnedState<T>
where
    T: Read<Box<T>> + ?Sized,
{
    /// Waits until the data of this state satisfy a given predicate, returning the data as a box
    ///
    /// This returns immediately if the current data already satisfy the predicate. Otherwise, it waits until the state
    /// is updated with data that satisfy the predicate. If you want to unconditionally wait until the state is updated,
    /// use [`wait_blocking`](OwnedState::wait_blocking).
    ///
    /// This returns the data for which the predicate returned `true`, causing the wait to finish. It produces a
    /// [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`wait_until_blocking`](OwnedState::wait_until_blocking) method.
    ///
    /// For example, to wait until the length of a slice reaches a given minimum:
    /// ```
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// use std::{io, thread};
    ///
    /// use wnf::{AsState, OwnedState};
    ///
    /// fn wait_until_len_at_least<S>(state: S, min_len: usize) -> io::Result<usize>
    /// where
    ///     S: AsState<Data = [u32]>,
    /// {
    ///     state
    ///         .as_state()
    ///         .wait_until_boxed_blocking(|slice| slice.len() >= min_len, Duration::MAX)
    ///         .map(|slice| slice.len())
    /// }
    ///
    /// let state = Arc::new(OwnedState::<[u32]>::create_temporary().expect("failed to create state"));
    /// state.set(&[]).expect("failed to set state data");
    ///
    /// {
    ///     let state = Arc::clone(&state);
    ///     thread::spawn(move || loop {
    ///         state
    ///             .apply_boxed(|slice| {
    ///                 let mut vec = slice.into_vec();
    ///                 vec.push(0);
    ///                 vec
    ///             })
    ///             .unwrap();
    ///
    ///         thread::sleep(Duration::from_millis(10));
    ///     });
    /// }
    ///
    /// let len = wait_until_len_at_least(&state, 10).expect("failed to wait for state update");
    /// assert!(len >= 10);
    /// ```
    ///
    /// This is a blocking method. If you are in an async context, use
    /// [`wait_until_boxed_async`](OwnedState::wait_until_boxed_async).
    ///
    /// # Errors
    /// Returns an error if querying, subscribing to or unsubscribing from the state fails or if the timeout has elapsed
    /// In the latter case, [`io::Error::kind`] returns [`ErrorKind::TimedOut`]
    pub fn wait_until_boxed_blocking<F>(&self, predicate: F, timeout: Duration) -> io::Result<Box<T>>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_boxed_blocking(predicate, timeout)
    }
}

impl<T> BorrowedState<'_, T>
where
    T: ?Sized,
{
    /// Waits until this state is updated
    ///
    /// See [`OwnedState::wait_blocking`]
    pub fn wait_blocking(self, timeout: Duration) -> io::Result<()> {
        self.raw.wait_blocking(timeout)
    }
}

impl<T> BorrowedState<'_, T>
where
    T: Read<T>,
{
    /// Waits until the data of this state satisfy a given predicate, returning the data
    ///
    /// See [`OwnedState::wait_until_blocking`]
    pub fn wait_until_blocking<F>(self, predicate: F, timeout: Duration) -> io::Result<T>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_blocking(predicate, timeout)
    }
}

impl<T> BorrowedState<'_, T>
where
    T: Read<Box<T>> + ?Sized,
{
    /// Waits until the data of this state satisfy a given predicate, returning the data as a box
    ///
    /// See [`OwnedState::wait_until_boxed_blocking`]
    pub fn wait_until_boxed_blocking<F>(self, predicate: F, timeout: Duration) -> io::Result<Box<T>>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_boxed_blocking(predicate, timeout)
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    /// Waits until this state is updated
    fn wait_blocking(self, timeout: Duration) -> io::Result<()> {
        let _: OpaqueData = self.cast().wait_until_blocking_internal(ChangedPredicate, timeout)?;
        Ok(())
    }
}

impl<T> RawState<T>
where
    T: Read<T>,
{
    /// Waits until the data of this state satisfy a given predicate, returning the data
    fn wait_until_blocking<F>(self, predicate: F, timeout: Duration) -> io::Result<T>
    where
        F: FnMut(&T) -> bool,
    {
        self.wait_until_blocking_internal(predicate, timeout)
    }
}

impl<T> RawState<T>
where
    T: Read<Box<T>> + ?Sized,
{
    /// Waits until the data of this state satisfy a given predicate, returning the data as a box
    fn wait_until_boxed_blocking<F>(self, predicate: F, timeout: Duration) -> io::Result<Box<T>>
    where
        F: FnMut(&T) -> bool,
    {
        self.wait_until_blocking_internal(predicate, timeout)
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    /// Waits until the data of this state satisfy a given predicate, returning the data as a value of type `D`
    ///
    /// The predicate is called once with [`PredicateStage::Initial`], then again with [`PredicateStage::Changed`] on
    /// every state update.
    ///
    /// If `T: Sized`, then `D` can be either `T` or `Box<T>`.
    /// If `T: !Sized`, then `D` must be `Box<T>`.
    fn wait_until_blocking_internal<D, F>(self, mut predicate: F, timeout: Duration) -> io::Result<D>
    where
        D: Borrow<T> + Send + 'static,
        F: Predicate<T>,
        T: Read<D>,
    {
        let (data, change_stamp) = self.query_as()?.into_data_change_stamp();

        if predicate.check(data.borrow(), PredicateStage::Initial) {
            return Ok(data);
        }

        let pair = Arc::new((Mutex::new(None), Condvar::new()));

        let subscription = {
            let pair = Arc::clone(&pair);

            self.subscribe(
                move |accessor: DataAccessor<'_, _>| {
                    let (mutex, condvar) = &*pair;
                    *mutex.lock().unwrap() = Some(accessor.get_as());
                    condvar.notify_one();
                },
                SeenChangeStamp::Value(change_stamp),
            )?
        };

        let (mutex, condvar) = &*pair;
        let (mut guard, timeout_result) = condvar
            .wait_timeout_while(mutex.lock().unwrap(), timeout, |result| match result.as_ref() {
                Some(Ok(data)) => !predicate.check(data.borrow(), PredicateStage::Changed),
                Some(Err(..)) => false,
                None => true,
            })
            .unwrap();

        subscription.unsubscribe()?;

        if timeout_result.timed_out() {
            Err(io::Error::new(
                ErrorKind::TimedOut,
                "waiting for state update timed out",
            ))
        } else {
            guard.take().unwrap()
        }
    }
}
