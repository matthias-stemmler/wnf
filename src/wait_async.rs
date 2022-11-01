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
    pub fn wait_async(&self) -> Wait<'_> {
        self.raw.wait_async()
    }
}

impl<T> OwnedState<T>
where
    T: Read<T>,
{
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
    pub fn wait_async(self) -> Wait<'a> {
        self.raw.wait_async()
    }
}

impl<'a, T> BorrowedState<'a, T>
where
    T: Read<T>,
{
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
    fn wait_async<'a>(self) -> Wait<'a> {
        Wait::new(self)
    }
}

impl<T> RawState<T>
where
    T: Read<T>,
{
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
    fn wait_until_boxed_async<'a, F>(self, predicate: F) -> WaitUntilBoxed<'a, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        WaitUntilBoxed::new(self, predicate)
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Wait<'a> {
    inner: WaitUntilInternal<'a, OpaqueData, OpaqueData, ChangedPredicate>,
}

impl Wait<'_> {
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
        Pin::new(&mut self.get_mut().inner).poll(cx).map_ok(|_| ())
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct WaitUntil<'a, T, F> {
    inner: WaitUntilInternal<'a, T, T, F>,
}

impl<F, T> WaitUntil<'_, T, F> {
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
        Pin::new(&mut self.get_mut().inner).poll(cx)
    }
}

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
        Pin::new(&mut self.get_mut().inner).poll(cx)
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
struct WaitUntilInternal<'a, T, D, F>
where
    T: ?Sized,
{
    future_state: Option<FutureState<'a, T, D, F>>,
}

impl<T, D, F> Unpin for WaitUntilInternal<'_, T, D, F> where T: ?Sized {}

#[derive(Debug)]
enum FutureState<'a, T, D, F>
where
    T: ?Sized,
{
    Initial {
        state: RawState<T>,
        predicate: F,
    },
    Waiting {
        predicate: F,
        shared_state: Arc<Mutex<SharedState<D>>>,
        subscription: Subscription<'a, WaitListener<D>>,
    },
}

#[derive(Debug)]
struct SharedState<D> {
    result: Option<io::Result<D>>,
    waker: Waker,
}

impl<D> SharedState<D> {
    fn from_waker(waker: Waker) -> Self {
        Self { result: None, waker }
    }
}

impl<D, F, T> WaitUntilInternal<'_, T, D, F>
where
    T: ?Sized,
{
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

#[derive(Debug)]
struct WaitListener<D> {
    shared_state: Arc<Mutex<SharedState<D>>>,
}

impl<D> WaitListener<D> {
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
