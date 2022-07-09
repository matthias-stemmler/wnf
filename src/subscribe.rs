use std::ffi::c_void;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::sync::Mutex;
use std::{fmt, mem, panic, ptr};

use thiserror::Error;
use tracing::{debug, trace_span};
use windows::core::GUID;
use windows::Win32::Foundation::{NTSTATUS, STATUS_SUCCESS};

use crate::data::WnfChangeStamp;
use crate::ntdll::NTDLL_TARGET;
use crate::read::WnfRead;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::state_name::WnfStateName;
use crate::{ntdll_sys, WnfReadError};

impl<T> OwnedWnfState<T>
where
    T: ?Sized,
{
    pub fn subscribe<F>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: FnMut(WnfDataAccessor<T>, WnfChangeStamp) + Send + ?Sized + 'static,
    {
        self.raw.subscribe(after_change_stamp, listener)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: ?Sized,
{
    pub fn subscribe<F>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: FnMut(WnfDataAccessor<T>, WnfChangeStamp) + Send + ?Sized + 'static,
    {
        self.raw.subscribe(after_change_stamp, listener)
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    pub fn subscribe<F>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: FnMut(WnfDataAccessor<T>, WnfChangeStamp) + Send + ?Sized + 'static,
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
            F: FnMut(WnfDataAccessor<T>, WnfChangeStamp) + Send + ?Sized + 'static,
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
                let scope = unsafe { WnfDataScope::new(buffer, buffer_size as usize) };

                context.with_listener(|listener| {
                    listener(scope.accessor(), change_stamp.into());
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

            Ok(WnfSubscriptionHandle::new(context, subscription))
        } else {
            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.state_name = %self.state_name,
                input.after_change_stamp = %after_change_stamp,
                "RtlSubscribeWnfStateChangeNotification",
            );

            Err(result.into())
        }
    }
}

#[derive(Debug)]
pub struct WnfDataAccessor<'a, T>
where
    T: ?Sized,
{
    scope: WnfDataScope<T>,
    _marker: PhantomData<&'a ()>,
}

#[derive(Debug)]
struct WnfDataScope<T>
where
    T: ?Sized,
{
    buffer: *const c_void,
    buffer_size: usize,
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

impl<T> WnfDataScope<T>
where
    T: ?Sized,
{
    unsafe fn new(buffer: *const c_void, buffer_size: usize) -> Self {
        Self {
            buffer,
            buffer_size,
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
}

impl<T> WnfDataAccessor<'_, T>
where
    T: WnfRead<T>,
{
    pub fn get(&self) -> Result<T, WnfReadError> {
        self.get_as()
    }
}

impl<T> WnfDataAccessor<'_, T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    pub fn get_boxed(&self) -> Result<Box<T>, WnfReadError> {
        self.get_as()
    }
}

impl<T> WnfDataAccessor<'_, T>
where
    T: ?Sized,
{
    pub(crate) fn get_as<D>(&self) -> Result<D, WnfReadError>
    where
        T: WnfRead<D>,
    {
        unsafe { T::from_buffer(self.scope.buffer, self.scope.buffer_size) }
    }
}

pub struct WnfSubscriptionHandle<'a, F>
where
    F: ?Sized,
{
    inner: Option<WnfSubscriptionHandleInner<F>>,
    _marker: PhantomData<&'a ()>,
}

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

pub(crate) struct WnfSubscriptionHandleInner<F>
where
    F: ?Sized,
{
    context: ManuallyDrop<Box<WnfSubscriptionContext<F>>>,
    subscription: u64,
}

impl<F> Debug for WnfSubscriptionHandleInner<F>
where
    F: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WnfSubscriptionHandleInner")
            .field("subscription", &self.subscription)
            .finish()
    }
}

pub(crate) struct WnfSubscriptionContext<F>(Mutex<Option<Box<F>>>)
where
    F: ?Sized;

impl<F> Debug for WnfSubscriptionContext<F>
where
    F: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WnfSubscriptionContext").field(&"..").finish()
    }
}

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

#[derive(Debug, Error, PartialEq)]
pub enum WnfSubscribeError {
    #[error("failed to subscribe to WNF state change: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

impl From<NTSTATUS> for WnfSubscribeError {
    fn from(result: NTSTATUS) -> Self {
        let err: windows::core::Error = result.into();
        err.into()
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfUnsubscribeError {
    #[error("failed to unsubscribe from WNF state change: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

impl From<NTSTATUS> for WnfUnsubscribeError {
    fn from(result: NTSTATUS) -> Self {
        let err: windows::core::Error = result.into();
        err.into()
    }
}
