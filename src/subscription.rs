use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::sync::Mutex;
use std::{fmt, mem};

use tracing::debug;

use crate::error::WnfUnsubscribeError;
use crate::ntdll::NTDLL_TARGET;
use crate::{ntdll_sys, WnfChangeStamp};

pub struct WnfSubscriptionHandle<'a, F>
where
    F: ?Sized,
{
    inner: Option<WnfSubscriptionHandleInner<F>>,
    _marker: PhantomData<&'a ()>,
}

// impl even if F is not Debug
impl<F> Debug for WnfSubscriptionHandle<'_, F>
where
    F: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WnfSubscriptionHandle")
            .field("subscription", &self.inner.as_ref().map(|inner| inner.subscription))
            .finish()
    }
}

impl<F> WnfSubscriptionHandle<'_, F>
where
    F: ?Sized,
{
    pub(crate) fn new(context: Box<WnfSubscriptionContext<F>>, subscription: u64) -> Self {
        Self {
            inner: Some(WnfSubscriptionHandleInner {
                context: ManuallyDrop::new(context),
                subscription,
            }),
            _marker: PhantomData,
        }
    }
}

#[derive(Debug)]
pub(crate) struct WnfSubscriptionHandleInner<F>
where
    F: ?Sized,
{
    context: ManuallyDrop<Box<WnfSubscriptionContext<F>>>,
    subscription: u64,
}

#[derive(Debug)]
pub(crate) struct WnfSubscriptionContext<F>(Mutex<Option<Box<F>>>)
where
    F: ?Sized;

impl<F> WnfSubscriptionContext<F>
where
    F: ?Sized,
{
    pub(crate) fn new(listener: Box<F>) -> Self {
        Self(Mutex::new(Some(listener)))
    }

    pub(crate) fn reset(&self) {
        let mut listener = match self.0.lock() {
            Ok(context) => context,
            Err(err) => err.into_inner(),
        };

        *listener = None;
    }

    pub(crate) fn with_listener(&self, op: impl FnOnce(&mut F)) {
        if let Ok(mut listener) = self.0.lock() {
            if let Some(listener) = listener.as_mut() {
                op(listener);
            }
        }
    }
}

impl<F> WnfSubscriptionHandle<'_, F>
where
    F: ?Sized,
{
    pub fn forget(self) {
        mem::forget(self);
    }

    // TODO wrap error with custom Debug implementation
    pub fn unsubscribe(mut self) -> Result<(), (WnfUnsubscribeError, Self)> {
        self.try_unsubscribe().map_err(|err| (err, self))
    }

    fn try_unsubscribe(&mut self) -> Result<(), WnfUnsubscribeError> {
        if let Some(inner) = self.inner.take() {
            let result = unsafe { ntdll_sys::RtlUnsubscribeWnfStateChangeNotification(inner.subscription) };

            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.subscription = inner.subscription,
                "RtlUnsubscribeWnfStateChangeNotification",
            );

            if result.is_ok() {
                ManuallyDrop::into_inner(inner.context);
            } else {
                self.inner = Some(inner);
            }

            result.ok()?;
        };

        Ok(())
    }
}

impl<F> Drop for WnfSubscriptionHandle<'_, F>
where
    F: ?Sized,
{
    fn drop(&mut self) {
        if self.try_unsubscribe().is_err() {
            if let Some(inner) = self.inner.take() {
                inner.context.reset();
            }
        }
    }
}

pub trait WnfStateChangeListener<T, A> {
    fn call(&mut self, data: Option<T>, change_stamp: WnfChangeStamp);
}

impl<F, T> WnfStateChangeListener<T, (Option<T>, WnfChangeStamp)> for F
where
    F: FnMut(Option<T>, WnfChangeStamp),
{
    fn call(&mut self, data: Option<T>, change_stamp: WnfChangeStamp) {
        self(data, change_stamp)
    }
}

impl<F, T> WnfStateChangeListener<T, (Option<T>,)> for F
where
    F: FnMut(Option<T>),
{
    fn call(&mut self, data: Option<T>, _: WnfChangeStamp) {
        self(data)
    }
}

impl<F, T> WnfStateChangeListener<T, ()> for F
where
    F: FnMut(),
{
    fn call(&mut self, _: Option<T>, _: WnfChangeStamp) {
        self()
    }
}
