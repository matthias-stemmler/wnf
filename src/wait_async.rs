use std::borrow::Borrow;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use crate::predicate::{ChangedPredicate, Predicate, PredicateStage};
use crate::state::RawWnfState;
use crate::subscribe::WnfSubscription;
use crate::{
    BorrowedWnfState, OwnedWnfState, WnfChangeStamp, WnfDataAccessor, WnfOpaqueData, WnfRead, WnfSeenChangeStamp,
    WnfStateListener,
};

impl<T> OwnedWnfState<T>
where
    T: ?Sized,
{
    pub fn wait_async(&self) -> WnfWait {
        self.raw.wait_async()
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfRead<T>,
{
    pub fn wait_until_async<F>(&self, predicate: F) -> WnfWaitUntil<T, F>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_async(predicate)
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    pub fn wait_until_boxed_async<F>(&self, predicate: F) -> WnfWaitUntilBoxed<T, F>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_boxed_async(predicate)
    }
}

impl<'a, T> BorrowedWnfState<'a, T>
where
    T: ?Sized,
{
    pub fn wait_async(self) -> WnfWait<'a> {
        self.raw.wait_async()
    }
}

impl<'a, T> BorrowedWnfState<'a, T>
where
    T: WnfRead<T>,
{
    pub fn wait_until_async<F>(self, predicate: F) -> WnfWaitUntil<'a, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_async(predicate)
    }
}

impl<'a, T> BorrowedWnfState<'a, T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    pub fn wait_until_boxed_async<F>(self, predicate: F) -> WnfWaitUntilBoxed<'a, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        self.raw.wait_until_boxed_async(predicate)
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    pub fn wait_async<'a>(self) -> WnfWait<'a> {
        WnfWait::new(self)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<T>,
{
    pub fn wait_until_async<'a, F>(self, predicate: F) -> WnfWaitUntil<'a, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        WnfWaitUntil::new(self, predicate)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    pub fn wait_until_boxed_async<'a, F>(self, predicate: F) -> WnfWaitUntilBoxed<'a, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        WnfWaitUntilBoxed::new(self, predicate)
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct WnfWait<'a> {
    inner: WnfWaitUntilInternal<'a, WnfOpaqueData, WnfOpaqueData, ChangedPredicate>,
}

impl WnfWait<'_> {
    fn new<T>(state: RawWnfState<T>) -> Self
    where
        T: ?Sized,
    {
        Self {
            inner: WnfWaitUntilInternal::new(state.cast(), ChangedPredicate),
        }
    }
}

impl Future for WnfWait<'_> {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().inner).poll(cx).map_ok(|_| ())
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct WnfWaitUntil<'a, T, F> {
    inner: WnfWaitUntilInternal<'a, T, T, F>,
}

impl<F, T> WnfWaitUntil<'_, T, F> {
    fn new(state: RawWnfState<T>, predicate: F) -> Self {
        Self {
            inner: WnfWaitUntilInternal::new(state, predicate),
        }
    }
}

impl<F, T> Future for WnfWaitUntil<'_, T, F>
where
    F: FnMut(&T) -> bool,
    T: WnfRead<T>,
{
    type Output = io::Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().inner).poll(cx)
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct WnfWaitUntilBoxed<'a, T, F>
where
    T: ?Sized,
{
    inner: WnfWaitUntilInternal<'a, T, Box<T>, F>,
}

impl<F, T> WnfWaitUntilBoxed<'_, T, F>
where
    T: ?Sized,
{
    fn new(state: RawWnfState<T>, predicate: F) -> Self {
        Self {
            inner: WnfWaitUntilInternal::new(state, predicate),
        }
    }
}

impl<T, F> Future for WnfWaitUntilBoxed<'_, T, F>
where
    F: FnMut(&T) -> bool,
    T: WnfRead<Box<T>> + ?Sized,
{
    type Output = io::Result<Box<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().inner).poll(cx)
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
struct WnfWaitUntilInternal<'a, T, D, F>
where
    T: ?Sized,
{
    future_state: Option<FutureState<'a, T, D, F>>,
}

impl<T, D, F> Unpin for WnfWaitUntilInternal<'_, T, D, F> where T: ?Sized {}

#[derive(Debug)]
enum FutureState<'a, T, D, F>
where
    T: ?Sized,
{
    Initial {
        state: RawWnfState<T>,
        predicate: F,
    },
    Waiting {
        predicate: F,
        shared_state: Arc<Mutex<SharedState<D>>>,
        subscription: WnfSubscription<'a, WaitListener<D>>,
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

impl<D, F, T> WnfWaitUntilInternal<'_, T, D, F>
where
    T: ?Sized,
{
    fn new(state: RawWnfState<T>, predicate: F) -> Self {
        Self {
            future_state: Some(FutureState::Initial { state, predicate }),
        }
    }
}

impl<D, F, T> Future for WnfWaitUntilInternal<'_, T, D, F>
where
    D: Borrow<T> + Send + 'static,
    F: Predicate<T>,
    T: WnfRead<D> + ?Sized,
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
                    let subscription = state.subscribe(WaitListener::new(Arc::clone(&shared_state), change_stamp))?;

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
    last_seen_change_stamp: WnfChangeStamp,
}

impl<D> WaitListener<D> {
    fn new(shared_state: Arc<Mutex<SharedState<D>>>, last_seen_change_stamp: WnfChangeStamp) -> Self {
        Self {
            shared_state,
            last_seen_change_stamp,
        }
    }
}

impl<T, D> WnfStateListener<T> for WaitListener<D>
where
    D: Send + 'static,
    T: WnfRead<D> + ?Sized,
{
    fn call(&mut self, accessor: WnfDataAccessor<T>) {
        let SharedState { result, ref waker } = &mut *self.shared_state.lock().unwrap();
        *result = Some(accessor.get_as());
        waker.wake_by_ref();
    }

    fn last_seen_change_stamp(&self) -> WnfSeenChangeStamp {
        WnfSeenChangeStamp::Value(self.last_seen_change_stamp)
    }
}
