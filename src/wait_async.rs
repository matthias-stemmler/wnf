//! Methods for asynchronously waiting for state updates

#![deny(unsafe_code)]

use std::borrow::Borrow;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use crate::data::OpaqueData;
use crate::predicate::{ChangedPredicate, Predicate, PredicateStage};
use crate::read::Read;
use crate::state::{BorrowedState, OwnedState, RawState};
use crate::subscribe::{DataAccessor, SeenChangeStamp, StateListener, Subscription};

impl<T> OwnedState<T>
where
    T: ?Sized,
{
    /// Waits until this state is updated
    ///
    /// This waits for *any* update to the state regardless of the value, even if the value is the same as the previous
    /// one. In order to wait until the state data satisfy a certain condition, use
    /// [`wait_until_async`](OwnedState::wait_until_async).
    ///
    /// Use this method if you want to wait for a state update *once*. In order to execute some logic on every state
    /// update, use the [`subscribe`](OwnedState::subscribe) method.
    ///
    /// This is an async method. If you are in an sync context, use [`wait_blocking`](OwnedState::wait_blocking).
    ///
    /// # Errors
    /// Returns an error if querying, subscribing to or unsubscribing from the state fails
    pub fn wait_async(&self) -> Wait<'_> {
        self.raw.wait_async()
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
    /// use [`wait_async`](OwnedState::wait_async).
    ///
    /// This returns the data for which the predicate returned `true`, causing the wait to finish. It produces an owned
    /// `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for `T: ?Sized`, use the
    /// [`wait_until_boxed_async`](OwnedState::wait_until_boxed_async) method.
    ///
    /// For example, to wait until the value of a state reaches a given minimum:
    /// ```
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// use std::{io, thread};
    ///
    /// use wnf::{AsState, OwnedState};
    ///
    /// async fn wait_until_at_least<S>(state: S, min_value: u32) -> io::Result<u32>
    /// where
    ///     S: AsState<Data = u32>,
    /// {
    ///     state.as_state().wait_until_async(|value| *value >= min_value).await
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state = Arc::new(OwnedState::create_temporary().expect("Failed to create state"));
    ///     state.set(&0).expect("Failed to set state data");
    ///
    ///     {
    ///         let state = Arc::clone(&state);
    ///         tokio::spawn(async move {
    ///             loop {
    ///                 state.apply(|value| value + 1).unwrap();
    ///                 tokio::time::sleep(Duration::from_millis(10)).await;
    ///             }
    ///         });
    ///     }
    ///
    ///     let value = wait_until_at_least(&state, 10)
    ///         .await
    ///         .expect("Failed to wait for state update");
    ///     assert!(value >= 10);
    /// }
    /// ```
    ///
    /// This is an async method. If you are in an sync context, use
    /// [`wait_until_blocking`](OwnedState::wait_until_blocking).
    ///
    /// # Errors
    /// Returns an error if querying, subscribing to or unsubscribing from the state fails
    pub fn wait_until_async<F>(&self, predicate: F) -> WaitUntil<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_async(predicate)
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
    /// use [`wait_async`](OwnedState::wait_async).
    ///
    /// This returns the data for which the predicate returned `true`, causing the wait to finish. It produces a
    /// [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`wait_until_async`](OwnedState::wait_until_async) method.
    ///
    /// For example, to wait until the length of a slice reaches a given minimum:
    /// ```
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// use std::{io, thread};
    ///
    /// use wnf::{AsState, OwnedState};
    ///
    /// async fn wait_until_len_at_least<S>(state: S, min_len: usize) -> io::Result<usize>
    /// where
    ///     S: AsState<Data = [u32]>,
    /// {
    ///     state
    ///         .as_state()
    ///         .wait_until_boxed_async(|slice| slice.len() >= min_len)
    ///         .await
    ///         .map(|slice| slice.len())
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state = Arc::new(OwnedState::<[u32]>::create_temporary().expect("Failed to create state"));
    ///     state.set(&[]).expect("Failed to set state data");
    ///
    ///     {
    ///         let state = Arc::clone(&state);
    ///         tokio::spawn(async move {
    ///             loop {
    ///                 state
    ///                     .apply_boxed(|slice| {
    ///                         let mut vec = slice.into_vec();
    ///                         vec.push(0);
    ///                         vec
    ///                     })
    ///                     .unwrap();
    ///
    ///                 tokio::time::sleep(Duration::from_millis(10)).await;
    ///             }
    ///         });
    ///     }
    ///
    ///     let len = wait_until_len_at_least(&state, 10)
    ///         .await
    ///         .expect("Failed to wait for state update");
    ///     assert!(len >= 10);
    /// }
    /// ```
    ///
    /// This is an async method. If you are in an sync context, use
    /// [`wait_until_boxed_blocking`](OwnedState::wait_until_boxed_blocking).
    ///
    /// # Errors
    /// Returns an error if querying, subscribing to or unsubscribing from the state fails
    pub fn wait_until_boxed_async<F>(&self, predicate: F) -> WaitUntilBoxed<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_boxed_async(predicate)
    }
}

impl<'a, T> BorrowedState<'a, T>
where
    T: ?Sized,
{
    /// Waits until this state is updated
    ///
    /// See [`OwnedState::wait_async`]
    pub fn wait_async(self) -> Wait<'a> {
        self.raw.wait_async()
    }
}

impl<'a, T> BorrowedState<'a, T>
where
    T: Read<T>,
{
    /// Waits until the data of this state satisfy a given predicate, returning the data
    ///
    /// See [`OwnedState::wait_until_async`]
    pub fn wait_until_async<F>(self, predicate: F) -> WaitUntil<'a, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_async(predicate)
    }
}

impl<'a, T> BorrowedState<'a, T>
where
    T: Read<Box<T>> + ?Sized,
{
    /// Waits until the data of this state satisfy a given predicate, returning the data as a box
    ///
    /// See [`OwnedState::wait_until_boxed_async`]
    pub fn wait_until_boxed_async<F>(self, predicate: F) -> WaitUntilBoxed<'a, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_boxed_async(predicate)
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    /// Waits until this state is updated
    fn wait_async<'a>(self) -> Wait<'a> {
        Wait::new(self)
    }
}

impl<T> RawState<T>
where
    T: Read<T>,
{
    /// Waits until the data of this state satisfy a given predicate, returning the data
    fn wait_until_async<'a, F>(self, predicate: F) -> WaitUntil<'a, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        WaitUntil::new(self, predicate)
    }
}

impl<T> RawState<T>
where
    T: Read<Box<T>> + ?Sized,
{
    /// Waits until the data of this state satisfy a given predicate, returning the data as a box
    fn wait_until_boxed_async<'a, F>(self, predicate: F) -> WaitUntilBoxed<'a, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        WaitUntilBoxed::new(self, predicate)
    }
}

/// Future returned by [`OwnedState::wait_async`]
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Wait<'a> {
    inner: WaitUntilInternal<'a, OpaqueData, OpaqueData, ChangedPredicate>,
}

impl Wait<'_> {
    /// Creates a new [`Wait<'_>`] future for the given raw state
    fn new<T>(state: RawState<T>) -> Self
    where
        T: ?Sized,
    {
        Self {
            inner: WaitUntilInternal::new(state.cast(), ChangedPredicate),
        }
    }
}

impl Future for Wait<'_> {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner_pinned = Pin::new(&mut self.get_mut().inner);
        inner_pinned.poll(cx).map_ok(|_| ())
    }
}

/// Future returned by [`OwnedState::wait_until_async`]
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct WaitUntil<'a, T, F> {
    inner: WaitUntilInternal<'a, T, T, F>,
}

impl<F, T> WaitUntil<'_, T, F> {
    /// Creates a new [`WaitUntil<'_, T, F>`] future for the given raw state and predicate
    fn new(state: RawState<T>, predicate: F) -> Self {
        Self {
            inner: WaitUntilInternal::new(state, predicate),
        }
    }
}

impl<F, T> Future for WaitUntil<'_, T, F>
where
    F: FnMut(&T) -> bool,
    T: Read<T>,
{
    type Output = io::Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner_pinned = Pin::new(&mut self.get_mut().inner);
        inner_pinned.poll(cx)
    }
}

/// Future returned by [`OwnedState::wait_until_boxed_async`]
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct WaitUntilBoxed<'a, T, F>
where
    T: ?Sized,
{
    inner: WaitUntilInternal<'a, T, Box<T>, F>,
}

impl<F, T> WaitUntilBoxed<'_, T, F>
where
    T: ?Sized,
{
    /// Creates a new [`WaitUntilBoxed<'_, T, F>`] future for the given raw state and predicate
    fn new(state: RawState<T>, predicate: F) -> Self {
        Self {
            inner: WaitUntilInternal::new(state, predicate),
        }
    }
}

impl<T, F> Future for WaitUntilBoxed<'_, T, F>
where
    F: FnMut(&T) -> bool,
    T: Read<Box<T>> + ?Sized,
{
    type Output = io::Result<Box<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner_pinned = Pin::new(&mut self.get_mut().inner);
        inner_pinned.poll(cx)
    }
}

/// Future generalizing the behavior of [`Wait<'_>`], [`WaitUntil<'_, T, F>`] and [`WaitUntilBoxed<'_, T, F>`]
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
struct WaitUntilInternal<'a, T, D, F>
where
    T: ?Sized,
{
    future_state: Option<FutureState<'a, T, D, F>>,
}

// This is not auto-implemented because `F` might be `!Unpin`
// We can implement it manually because `F` is never pinned, i.e. pinning is non-structural for `F`
// See <https://doc.rust-lang.org/std/pin/index.html#pinning-is-not-structural-for-field>
impl<T, D, F> Unpin for WaitUntilInternal<'_, T, D, F> where T: ?Sized {}

/// State of the [`WaitUntilInternal<'a, T, D, F>`] future
#[derive(Debug)]
enum FutureState<'a, T, D, F>
where
    T: ?Sized,
{
    /// Future has not been polled
    Initial { state: RawState<T>, predicate: F },

    /// Future is waiting for state update
    Waiting {
        predicate: F,
        shared_state: Arc<Mutex<SharedState<D>>>,
        subscription: Subscription<'a, WaitListener<D>>,
    },
}

/// Shared state between the polling thread and the waking thread
#[derive(Debug)]
struct SharedState<D> {
    result: Option<io::Result<D>>,
    waker: Waker,
}

impl<D> SharedState<D> {
    /// Creates a new [`SharedState<D>`] from the given waker
    fn from_waker(waker: Waker) -> Self {
        Self { result: None, waker }
    }
}

impl<D, F, T> WaitUntilInternal<'_, T, D, F>
where
    T: ?Sized,
{
    /// Creates a new [`WaitUntilInternal<'_, T, D, F>`] future for the given raw state and predicate
    fn new(state: RawState<T>, predicate: F) -> Self {
        Self {
            future_state: Some(FutureState::Initial { state, predicate }),
        }
    }
}

impl<D, F, T> Future for WaitUntilInternal<'_, T, D, F>
where
    D: Borrow<T> + Send + 'static,
    F: Predicate<T>,
    T: Read<D> + ?Sized,
{
    type Output = io::Result<D>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future_state = Some(
            match self.future_state.take().expect("Future polled after it has completed") {
                FutureState::Initial { state, mut predicate } => {
                    let (data, change_stamp) = state.query_as()?.into_data_change_stamp();

                    if predicate.check(data.borrow(), PredicateStage::Initial) {
                        return Poll::Ready(Ok(data));
                    }

                    let shared_state = Arc::new(Mutex::new(SharedState::from_waker(cx.waker().clone())));
                    let subscription = state.subscribe(
                        WaitListener::new(Arc::clone(&shared_state)),
                        SeenChangeStamp::Value(change_stamp),
                    )?;

                    FutureState::Waiting {
                        predicate,
                        shared_state,
                        subscription,
                    }
                }

                FutureState::Waiting {
                    mut predicate,
                    shared_state,
                    subscription,
                } => {
                    let mut guard = shared_state.lock().unwrap();
                    let SharedState { result, waker } = &mut *guard;

                    match result.take().unwrap() {
                        Ok(data) if !predicate.check(data.borrow(), PredicateStage::Changed) => {
                            if !waker.will_wake(cx.waker()) {
                                *waker = cx.waker().clone();
                            }
                        }

                        result => {
                            subscription.unsubscribe()?;
                            return Poll::Ready(Ok(result?));
                        }
                    }

                    drop(guard);

                    FutureState::Waiting {
                        predicate,
                        shared_state,
                        subscription,
                    }
                }
            },
        );

        Poll::Pending
    }
}

/// State listener that saves the result of accessing the state data and wakes a waker
///
/// This is a type that can be named rather than an anonymous closure type so that it can be stored in a
/// [`FutureState<'a, T, D, F>`] without using a trait object.
#[derive(Debug)]
struct WaitListener<D> {
    shared_state: Arc<Mutex<SharedState<D>>>,
}

impl<D> WaitListener<D> {
    /// Creates a new [`WaitListener<D>`] with the given shared state
    fn new(shared_state: Arc<Mutex<SharedState<D>>>) -> Self {
        Self { shared_state }
    }
}

impl<T, D> StateListener<T> for WaitListener<D>
where
    D: Send + 'static,
    T: Read<D> + ?Sized,
{
    fn call(&mut self, accessor: DataAccessor<'_, T>) {
        let SharedState { result, ref waker } = &mut *self.shared_state.lock().unwrap();
        *result = Some(accessor.get_as());
        waker.wake_by_ref();
    }
}