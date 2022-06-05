use std::alloc::Layout;
use std::ffi::c_void;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::sync::Mutex;
use std::{alloc, fmt, mem, panic, ptr};

use thiserror::Error;
use tracing::{debug, trace_span};
use windows::core::GUID;
use windows::Win32::Foundation::{NTSTATUS, STATUS_SUCCESS};

use crate::bytes::CheckedBitPattern;
use crate::callback::WnfCallback;
use crate::data::WnfChangeStamp;
use crate::ntdll::NTDLL_TARGET;
use crate::ntdll_sys;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::state_name::WnfStateName;

impl<T> OwnedWnfState<T>
where
    T: CheckedBitPattern,
{
    pub fn subscribe<F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: WnfCallback<T, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        self.raw.subscribe(after_change_stamp, listener)
    }

    pub fn subscribe_slice_boxed<F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: WnfCallback<Box<T>, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        self.raw.subscribe_boxed(after_change_stamp, listener)
    }

    pub fn subscribe_slice<F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: WnfCallback<Box<[T]>, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        self.raw.subscribe_slice(after_change_stamp, listener)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: CheckedBitPattern,
{
    pub fn subscribe<F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: WnfCallback<T, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        self.raw.subscribe(after_change_stamp, listener)
    }

    pub fn subscribe_boxed<F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: WnfCallback<Box<T>, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        self.raw.subscribe_boxed(after_change_stamp, listener)
    }

    pub fn subscribe_slice<F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: WnfCallback<Box<[T]>, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        self.raw.subscribe_slice(after_change_stamp, listener)
    }
}

impl<T> RawWnfState<T>
where
    T: CheckedBitPattern,
{
    pub fn subscribe<F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: WnfCallback<T, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        self.subscribe_internal::<Value<T>, F, ArgsValid, ArgsInvalid>(after_change_stamp, listener)
    }

    pub fn subscribe_boxed<F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: WnfCallback<Box<T>, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        self.subscribe_internal::<Boxed<T>, F, ArgsValid, ArgsInvalid>(after_change_stamp, listener)
    }

    pub fn subscribe_slice<F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: WnfCallback<Box<[T]>, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        self.subscribe_internal::<BoxedSlice<T>, F, ArgsValid, ArgsInvalid>(after_change_stamp, listener)
    }

    fn subscribe_internal<B, F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        B: FromByteBuffer,
        F: WnfCallback<B::Data, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        extern "system" fn callback<B, F, ArgsValid, ArgsInvalid>(
            state_name: u64,
            change_stamp: u32,
            _type_id: *const GUID,
            context: *mut c_void,
            buffer: *const c_void,
            buffer_size: u32,
        ) -> NTSTATUS
        where
            B: FromByteBuffer,
            F: WnfCallback<B::Data, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
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
                let maybe_data = unsafe { B::from_byte_buffer(buffer, buffer_size) };

                context.with_listener(|listener| match maybe_data {
                    Some(data) => {
                        listener.call_valid(data, change_stamp.into());
                    }
                    None => {
                        listener.call_invalid(change_stamp.into());
                    }
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
                callback::<B, F, ArgsValid, ArgsInvalid>,
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

trait FromByteBuffer {
    type Data;

    unsafe fn from_byte_buffer(ptr: *const c_void, size: u32) -> Option<Self::Data>;
}

#[derive(Debug)]
struct Value<T>(PhantomData<fn() -> T>);

impl<T> FromByteBuffer for Value<T>
where
    T: CheckedBitPattern,
{
    type Data = T;

    unsafe fn from_byte_buffer(ptr: *const c_void, size: u32) -> Option<T> {
        if size as usize != mem::size_of::<T::Bits>() {
            return None;
        }

        let bits: T::Bits = ptr::read_unaligned(ptr.cast());

        if T::is_valid_bit_pattern(&bits) {
            Some(*(&bits as *const T::Bits as *const T))
        } else {
            None
        }
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

#[derive(Debug)]
struct Boxed<T>(PhantomData<fn() -> Box<T>>);

impl<T> FromByteBuffer for Boxed<T>
where
    T: CheckedBitPattern,
{
    type Data = Box<T>;

    unsafe fn from_byte_buffer(ptr: *const c_void, size: u32) -> Option<Box<T>> {
        if size as usize != mem::size_of::<T::Bits>() {
            return None;
        }

        let bits = if mem::size_of::<T::Bits>() == 0 {
            Box::new(mem::zeroed())
        } else {
            let layout = Layout::new::<T::Bits>();
            let data = alloc::alloc(layout) as *mut T::Bits;
            ptr::copy_nonoverlapping(ptr as *const T::Bits, data as *mut T::Bits, 1);
            Box::from_raw(data)
        };

        T::is_valid_bit_pattern(&bits).then(|| Box::from_raw(Box::into_raw(bits) as *mut T))
    }
}

#[derive(Debug)]
struct BoxedSlice<T>(PhantomData<fn() -> Box<[T]>>);

impl<T> FromByteBuffer for BoxedSlice<T>
where
    T: CheckedBitPattern,
{
    type Data = Box<[T]>;

    unsafe fn from_byte_buffer(ptr: *const c_void, size: u32) -> Option<Box<[T]>> {
        if mem::size_of::<T>() == 0 {
            return (size == 0).then(|| Vec::new().into_boxed_slice());
        }

        if size as usize % mem::size_of::<T>() != 0 {
            return None;
        }

        let len = size as usize / mem::size_of::<T>();
        let mut data = Vec::with_capacity(len);
        ptr::copy_nonoverlapping(ptr.cast(), data.as_mut_ptr(), len);
        data.set_len(len);

        if data.iter().all(T::is_valid_bit_pattern) {
            let mut data = ManuallyDrop::new(data);
            let data = Vec::from_raw_parts(data.as_mut_ptr() as *mut T, data.len(), data.capacity());
            Some(data.into_boxed_slice())
        } else {
            None
        }
    }
}
