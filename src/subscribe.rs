use std::ffi::c_void;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::sync::Mutex;
use std::{fmt, io, mem, panic, ptr};

use tracing::{debug, trace_span};
use windows::core::GUID;
use windows::Win32::Foundation::{NTSTATUS, STATUS_SUCCESS};

use crate::data::WnfChangeStamp;
use crate::ntdll::NTDLL_TARGET;
use crate::read::WnfRead;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::state_name::WnfStateName;
use crate::{ntdll_sys, WnfStampedData};

pub trait WnfStateListener<T>: Send + 'static
where
    T: ?Sized,
{
    fn call(&mut self, accessor: WnfDataAccessor<T>);
}

impl<F, T> WnfStateListener<T> for F
where
    F: FnMut(WnfDataAccessor<T>) + Send + 'static,
    T: ?Sized,
{
    fn call(&mut self, accessor: WnfDataAccessor<T>) {
        self(accessor)
    }
}

impl<T> OwnedWnfState<T>
where
    T: ?Sized,
{
    pub fn subscribe<F>(&self, after_change_stamp: WnfChangeStamp, listener: F) -> io::Result<WnfSubscription<F>>
    where
        F: WnfStateListener<T>,
    {
        self.raw.subscribe(after_change_stamp, listener)
    }
}

impl<'a, T> BorrowedWnfState<'a, T>
where
    T: ?Sized,
{
    pub fn subscribe<F>(&self, after_change_stamp: WnfChangeStamp, listener: F) -> io::Result<WnfSubscription<'a, F>>
    where
        F: WnfStateListener<T>,
    {
        self.raw.subscribe(after_change_stamp, listener)
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    // Note: if unsubscribe fails, will leak data the size of `Option<F>`
    // if that's too much, box the listener
    pub fn subscribe<'a, F>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: F,
    ) -> io::Result<WnfSubscription<'a, F>>
    where
        F: WnfStateListener<T>,
    {
        extern "system" fn callback<F, T>(
            state_name: u64,
            change_stamp: u32,
            _type_id: *const GUID,
            context: *mut c_void,
            buffer: *const c_void,
            buffer_size: u32,
        ) -> NTSTATUS
        where
            F: WnfStateListener<T>,
            T: ?Sized,
        {
            let _ = panic::catch_unwind(|| {
                let span = trace_span!(
                    target: NTDLL_TARGET,
                    "WnfUserCallback",
                    input.state_name = %WnfStateName::from_opaque_value(state_name),
                    input.change_stamp = change_stamp,
                    input.buffer_size = buffer_size
                );
                let _enter = span.enter();

                let context: &WnfSubscriptionContext<F> = unsafe { &*context.cast() };
                let scope = unsafe { WnfDataScope::new(buffer, buffer_size as usize, change_stamp.into()) };

                context.with_listener(|listener| {
                    listener.call(scope.accessor());
                });
            });

            STATUS_SUCCESS
        }

        let mut subscription = 0;
        let context = Box::new(WnfSubscriptionContext::new(listener));

        let result = unsafe {
            ntdll_sys::RtlSubscribeWnfStateChangeNotification(
                &mut subscription,
                self.state_name.opaque_value(),
                after_change_stamp.into(),
                callback::<F, T>,
                &*context as *const _ as *mut c_void,
                ptr::null(),
                0,
                0,
            )
        };

        if result.is_ok() {
            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.state_name = %self.state_name,
                input.after_change_stamp = %after_change_stamp,
                output.subscription = subscription,
                "RtlSubscribeWnfStateChangeNotification",
            );

            Ok(WnfSubscription::new(context, subscription))
        } else {
            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.state_name = %self.state_name,
                input.after_change_stamp = %after_change_stamp,
                "RtlSubscribeWnfStateChangeNotification",
            );

            Err(io::Error::from_raw_os_error(result.0))
        }
    }
}

pub struct WnfDataAccessor<'a, T>
where
    T: ?Sized,
{
    scope: WnfDataScope<T>,
    _marker: PhantomData<&'a ()>,
}

impl<T> Copy for WnfDataAccessor<'_, T> where T: ?Sized {}

impl<T> Clone for WnfDataAccessor<'_, T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Debug for WnfDataAccessor<'_, T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WnfDataAccessor").field("scope", &self.scope).finish()
    }
}

struct WnfDataScope<T>
where
    T: ?Sized,
{
    buffer: *const c_void,
    buffer_size: usize,
    change_stamp: WnfChangeStamp,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Copy for WnfDataScope<T> where T: ?Sized {}

impl<T> Clone for WnfDataScope<T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Debug for WnfDataScope<T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WnfDataScope")
            .field("buffer", &self.buffer)
            .field("buffer_size", &self.buffer_size)
            .field("change_stamp", &self.change_stamp)
            .finish()
    }
}

impl<T> WnfDataScope<T>
where
    T: ?Sized,
{
    unsafe fn new(buffer: *const c_void, buffer_size: usize, change_stamp: WnfChangeStamp) -> Self {
        Self {
            buffer,
            buffer_size,
            change_stamp,
            _marker: PhantomData,
        }
    }

    fn accessor(&self) -> WnfDataAccessor<T> {
        WnfDataAccessor {
            scope: *self,
            _marker: PhantomData,
        }
    }

    fn cast<U>(self) -> WnfDataScope<U> {
        WnfDataScope {
            buffer: self.buffer,
            buffer_size: self.buffer_size,
            change_stamp: self.change_stamp,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> WnfDataAccessor<'a, T>
where
    T: ?Sized,
{
    pub fn cast<U>(self) -> WnfDataAccessor<'a, U> {
        WnfDataAccessor {
            scope: self.scope.cast(),
            _marker: PhantomData,
        }
    }

    pub fn change_stamp(self) -> WnfChangeStamp {
        self.scope.change_stamp
    }
}

impl<T> WnfDataAccessor<'_, T>
where
    T: WnfRead<T>,
{
    pub fn get(self) -> io::Result<T> {
        self.get_as()
    }

    pub fn query(self) -> io::Result<WnfStampedData<T>> {
        self.query_as()
    }
}

impl<T> WnfDataAccessor<'_, T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    pub fn get_boxed(self) -> io::Result<Box<T>> {
        self.get_as()
    }

    pub fn query_boxed(self) -> io::Result<WnfStampedData<Box<T>>> {
        self.query_as()
    }
}

impl<T> WnfDataAccessor<'_, T>
where
    T: ?Sized,
{
    pub(crate) fn get_as<D>(self) -> io::Result<D>
    where
        T: WnfRead<D>,
    {
        unsafe { T::from_buffer(self.scope.buffer, self.scope.buffer_size) }
    }

    pub(crate) fn query_as<D>(self) -> io::Result<WnfStampedData<D>>
    where
        T: WnfRead<D>,
    {
        Ok(WnfStampedData::from_data_change_stamp(
            self.get_as()?,
            self.scope.change_stamp,
        ))
    }
}

#[must_use]
pub struct WnfSubscription<'a, F> {
    inner: Option<WnfSubscriptionInner<F>>,
    _marker: PhantomData<&'a ()>,
}

impl<F> Debug for WnfSubscription<'_, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WnfSubscription")
            .field("subscription", &self.inner.as_ref().map(|inner| inner.subscription))
            .finish()
    }
}

impl<F> WnfSubscription<'_, F> {
    pub(crate) fn new(context: Box<WnfSubscriptionContext<F>>, subscription: u64) -> Self {
        Self {
            inner: Some(WnfSubscriptionInner {
                context: ManuallyDrop::new(context),
                subscription,
            }),
            _marker: PhantomData,
        }
    }
}

pub(crate) struct WnfSubscriptionInner<F> {
    context: ManuallyDrop<Box<WnfSubscriptionContext<F>>>,
    subscription: u64,
}

impl<F> Debug for WnfSubscriptionInner<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WnfSubscriptionInner")
            .field("subscription", &self.subscription)
            .finish()
    }
}

pub(crate) struct WnfSubscriptionContext<F>(Mutex<Option<F>>);

impl<F> Debug for WnfSubscriptionContext<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WnfSubscriptionContext").field(&"..").finish()
    }
}

impl<F> WnfSubscriptionContext<F> {
    pub(crate) fn new(listener: F) -> Self {
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

impl<F> WnfSubscription<'_, F> {
    pub fn forget(self) {
        mem::forget(self);
    }

    pub fn unsubscribe(mut self) -> io::Result<()> {
        self.try_unsubscribe()
    }

    fn try_unsubscribe(&mut self) -> io::Result<()> {
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
                inner.context.reset();
            }

            result.ok()?;
        };

        Ok(())
    }
}

impl<F> Drop for WnfSubscription<'_, F> {
    fn drop(&mut self) {
        let _ = self.try_unsubscribe();
    }
}
