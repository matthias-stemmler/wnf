use std::marker::PhantomData;
use std::mem;
use std::mem::ManuallyDrop;
use std::sync::Mutex;

use crate::error::WnfUnsubscribeError;
use crate::ntdll_sys;

#[derive(Debug)]
pub struct WnfSubscriptionHandle<'a, F: ?Sized> {
    inner: Option<WnfSubscriptionHandleInner<F>>,
    _marker: PhantomData<&'a ()>,
}

impl<F: ?Sized> WnfSubscriptionHandle<'_, F> {
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
pub(crate) struct WnfSubscriptionHandleInner<F: ?Sized> {
    context: ManuallyDrop<Box<WnfSubscriptionContext<F>>>,
    subscription: u64,
}

#[derive(Debug)]
pub(crate) struct WnfSubscriptionContext<F: ?Sized>(Mutex<Option<Box<F>>>);

impl<F: ?Sized> WnfSubscriptionContext<F> {
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

impl<F: ?Sized> WnfSubscriptionHandle<'_, F> {
    pub fn forget(self) {
        mem::forget(self);
    }

    pub fn unsubscribe(mut self) -> Result<(), (WnfUnsubscribeError, Self)> {
        self.try_unsubscribe().map_err(|err| (err, self))
    }

    fn try_unsubscribe(&mut self) -> Result<(), WnfUnsubscribeError> {
        if let Some(inner) = self.inner.take() {
            let result = unsafe { ntdll_sys::RtlUnsubscribeWnfStateChangeNotification(inner.subscription) };

            if result.is_ok() {
                ManuallyDrop::into_inner(inner.context);
            } else {
                self.inner = Some(inner);
            }
        };

        Ok(())
    }
}

impl<F: ?Sized> Drop for WnfSubscriptionHandle<'_, F> {
    fn drop(&mut self) {
        if self.try_unsubscribe().is_err() {
            if let Some(inner) = self.inner.take() {
                inner.context.reset();
            }
        }
    }
}
