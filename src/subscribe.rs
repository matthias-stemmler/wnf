//! Methods for subscribing to WNF state changes

use std::ffi::c_void;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::sync::Mutex;
use std::{fmt, io, mem, panic, ptr};

use tracing::{debug, trace_span};
use windows::core::GUID;
use windows::Win32::Foundation::{NTSTATUS, STATUS_SUCCESS};

use crate::data::{WnfChangeStamp, WnfStampedData};
use crate::ntapi;
use crate::read::WnfRead;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::state_name::WnfStateName;

/// The change stamp that a state listener has last seen
///
/// The [`OwnedWnfState::subscribe`] and [`BorrowedWnfState::subscribe`] methods expect an argument of this type to
/// indicate what change stamp of the state the listener has last seen. The listener will then only be notified about
/// updates with a (strictly) larger change stamp, i.e. updates that happen after the one it has last seen.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WnfSeenChangeStamp {
    /// Indicates that a listener has not seen any change stamp
    ///
    /// When subscribing with this value, the listener will be notified once about the current data of the state (only
    /// if any data has been written to the state yet) and then again once about each future state update.
    ///
    /// This is most useful for listeners that track the data of the state, e.g. for showing it in a UI, because it
    /// eliminates the need for a separate query for the current data before subscribing.
    #[default]
    None,

    /// Indicates that a listener has seen the current change stamp
    ///
    /// When subscribing with this value, the listener will be notified once about each future state update, but not
    /// about the current data of the state.
    ///
    /// This is most useful for listeners that treat updates to the state data as events, e.g. for logging, because it
    /// ensures that they are only called when an actual update happens.
    Current,

    /// Indicates that a listener has seen a change stamp with a particular value
    ///
    /// When subscribing with this value, the listener will be notified once about each state update whose change stamp
    /// is strictly larger than the given change stamp. The earliest possible notification is for the current data of
    /// the state (only if any data has been written to the state yet), i.e. it will never be notified about past state
    /// updates regardless of the passed value.
    ///
    /// This is most useful if you have queried the state data before and are already holding a change stamp.
    Value(WnfChangeStamp),
}

/// Types capable of listening to WNF state updates
///
/// Note that there is a blanket implementation of this trait for all closure types
/// `F: FnMut(WnfDataAccessor<T>)`, so you usually don't need to implement this trait for your own types. It is useful,
/// however, if you need a WNF state listener whose type you can name explicitly. Since closure types are anonymous, you
/// can instead define your own type and implement [`WnfStateListener<T>`] for it.
pub trait WnfStateListener<T>
where
    T: ?Sized,
{
    /// Calls this WNF state listener
    ///
    /// The provided [`WnfDataAccessor<T>`] can be used to obtain the state data at the time the update took place.
    fn call(&mut self, accessor: WnfDataAccessor<T>);
}

impl<F, T> WnfStateListener<T> for F
where
    F: FnMut(WnfDataAccessor<T>),
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
    /// Subscribes the given state listener to this [`OwnedWnfState<T>`]
    ///
    /// The `last_seen_change_stamp` argument can be used to indicate what change stamp of the state you have last seen,
    /// which has an impact on which state updates the listener is notified about. See [`WnfSeenChangeStamp`] for the
    /// available options.
    ///
    /// Note that the listener is automatically unsubscribed when the returned [`WnfSubscription<'a, F>`] is dropped. In
    /// this case, errors while unsubscribing are silently ignored. If you want to handle them explicitly, use the
    /// [`WnfSubscription::unsubscribe`] method, which returns an [`io::Result<()>`].
    ///
    /// In any case, the listener will not be called anymore after unsubscribing, even when there is an error. However,
    /// in order to maintain memory safety, in the case of an error a value the size of a [`Mutex<Option<F>>`] is leaked
    /// on the heap. This should be fine in most cases, especially when `F` is small. Otherwise consider using a boxed
    /// closure.
    pub fn subscribe<F>(
        &self,
        listener: F,
        last_seen_change_stamp: WnfSeenChangeStamp,
    ) -> io::Result<WnfSubscription<F>>
    where
        F: WnfStateListener<T> + Send + 'static,
    {
        self.raw.subscribe(listener, last_seen_change_stamp)
    }
}

impl<'a, T> BorrowedWnfState<'a, T>
where
    T: ?Sized,
{
    /// Subscribes the given state listener to this [`BorrowedWnfState<'a, T>`]
    ///
    /// The `last_seen_change_stamp` argument can be used to indicate what change stamp of the state you have last seen,
    /// which has an impact on which state updates the listener is notified about. See [`WnfSeenChangeStamp`] for the
    /// available options.
    ///
    /// Note that the listener is automatically unsubscribed when the returned [`WnfSubscription<'a, F>`] is dropped. In
    /// this case, errors while unsubscribing are silently ignored. If you want to handle them explicitly, use the
    /// [`WnfSubscription::unsubscribe`] method, which returns an [`io::Result<()>`].
    ///
    /// In any case, the listener will not be called anymore after unsubscribing, even when there is an error. However,
    /// in order to maintain memory safety, in the case of an error a value the size of a [`Mutex<Option<F>>`] is leaked
    /// on the heap. This should be fine in most cases, especially when `F` is small. Otherwise consider using a boxed
    /// closure.
    pub fn subscribe<F>(
        &self,
        listener: F,
        last_seen_change_stamp: WnfSeenChangeStamp,
    ) -> io::Result<WnfSubscription<'a, F>>
    where
        F: WnfStateListener<T> + Send + 'static,
    {
        self.raw.subscribe(listener, last_seen_change_stamp)
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    /// Subscribes the given WNF state listener to this [`RawWnfState<T>`].
    pub(crate) fn subscribe<'a, F>(
        &self,
        listener: F,
        last_seen_change_stamp: WnfSeenChangeStamp,
    ) -> io::Result<WnfSubscription<'a, F>>
    where
        F: WnfStateListener<T> + Send + 'static,
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
            F: WnfStateListener<T> + Send + 'static,
            T: ?Sized,
        {
            let _ = panic::catch_unwind(|| {
                let span = trace_span!(
                    target: ntapi::TRACING_TARGET,
                    "WnfUserCallback",
                    input.state_name = %WnfStateName::from_opaque_value(state_name),
                    input.change_stamp = change_stamp,
                    input.buffer_size = buffer_size
                );
                let _enter = span.enter();

                // SAFETY:
                // (1) By the assumption on `RtlSubscribeWnfStateChangeNotification`, `context` is the pointer passed in
                // the fifth argument of some successful call to that function. Let `subscription_handle` be the
                // subscription handle returned from that call. Then `context` points to a `WnfSubscriptionContext<F>`
                // holding this `subscription_handle`.
                //
                // (2) By
                // - the assumptions on `RtlUnsubscribeWnfStateChangeNotification`,
                // - the fact that `callback` was called with `context` and
                // - the fact that `context` is unique among all calls to `RtlSubscribeWnfStateChangeNotification` with
                //   `callback`,
                // we know that either
                // (a) there has been no call to `RtlUnsubscribeWnfStateChangeNotification` with `subscription_handle`
                //     yet, or
                // (b) all calls to `RtlUnsubscribeWnfStateChangeNotification` with `subscription_handle` have failed
                //
                // From (1), it follows that the `WnfSubscriptionContext<F>` was not dropped in the `subscribe` method
                // but returned wrapped in a `WnfSubscription<'a, F>`.
                //
                // In case (a) we know that the `WnfSubscription<'a, F>` has not been dropped because dropping it would
                // have called `RtlUnsubscribeWnfStateChangeNotification` with `subscription_handle`. Hence the
                // `WnfSubscriptionContext<F>` hasn't been dropped either.
                //
                // In case (b) the `WnfSubscription<'a, F>` has been dropped but the `WnfSubscriptionContext<F>` it
                // contains has been leaked (see comment below).
                //
                // In any case, `context` points to a valid `WnfSubscriptionContext<F>`.
                //
                // (3) We may be on a different thread than the one that created the `WnfSubscriptionContext<F>`, but
                // `F: Send` implies `WnfSubscriptionContext<F>: Send`.
                //
                // (4) `F` outlives the lifetime of the produced reference because `F: 'static`.
                let context: &WnfSubscriptionContext<F> = unsafe { &*context.cast() };

                // SAFETY:
                // - By the assumption on `RtlSubscribeWnfStateChangeNotification`, the assumption on `WnfUserCallback`
                //   is satisfied
                // - As `data` is dropped before `callback` returns, the assumption on `WnfUserCallback` then implies
                //   the safety conditions of `WnfScopedData::new`
                let data = unsafe { WnfScopedData::new(buffer, buffer_size as usize, change_stamp.into()) };

                context.with_listener(|listener| {
                    listener.call(data.accessor());
                });
            });

            STATUS_SUCCESS
        }

        let change_stamp = match last_seen_change_stamp {
            WnfSeenChangeStamp::None => WnfChangeStamp::initial(),
            WnfSeenChangeStamp::Current => self.change_stamp()?,
            WnfSeenChangeStamp::Value(value) => value,
        };

        let mut subscription_handle = ptr::null_mut();
        let context = Box::new(WnfSubscriptionContext::new(listener));

        // SAFETY:
        // - The pointer in the first argument is valid for writes of `*mut c_void` because it comes from a live mutable
        //   reference
        // - The function pointed to by the pointer in the fourth argument does not unwind because it catches all
        //   unwinding panics
        // - The pointer in the fifth argument is either a null pointer or points to a valid `GUID` by the guarantees of
        //   `TypeId::as_ptr`
        let result = unsafe {
            ntapi::RtlSubscribeWnfStateChangeNotification(
                &mut subscription_handle,
                self.state_name.opaque_value(),
                change_stamp.into(),
                callback::<F, T>,
                &*context as *const WnfSubscriptionContext<F> as *mut c_void,
                self.type_id.as_ptr(),
                0,
                0,
            )
        };

        if result.is_ok() {
            let subscription = WnfSubscription::new(context, subscription_handle);

            debug!(
                target: ntapi::TRACING_TARGET,
                ?result,
                input.state_name = %self.state_name,
                input.change_stamp = %change_stamp,
                input.type_id = %self.type_id,
                output.subscription_handle = subscription_handle as u64,
                "RtlSubscribeWnfStateChangeNotification",
            );

            Ok(subscription)
        } else {
            debug!(
                target: ntapi::TRACING_TARGET,
                ?result,
                input.state_name = %self.state_name,
                input.change_stamp = %change_stamp,
                input.type_id = %self.type_id,
                "RtlSubscribeWnfStateChangeNotification",
            );

            Err(io::Error::from_raw_os_error(result.0))
        }
    }
}

/// Handle to state data passed to state listeners
///
/// Listeners receive a [`WnfDataAccessor<'a, T>`] in their [`WnfStateListener::call`] method. It can be used to obtain
/// the state data at the time the update took place.
///
/// The lifetime parameter `'a` ties a [`WnfDataAccessor<'a, T>`] to the lifetime of the state data, which is only valid
/// within the scope of the call to the listener.
pub struct WnfDataAccessor<'a, T>
where
    T: ?Sized,
{
    data: WnfScopedData<T>,
    _marker: PhantomData<&'a ()>,
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Copy`
impl<T> Copy for WnfDataAccessor<'_, T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Clone`
impl<T> Clone for WnfDataAccessor<'_, T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Debug`
impl<T> Debug for WnfDataAccessor<'_, T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WnfDataAccessor").field("data", &self.data).finish()
    }
}

/// State data that is only valid within a certain scope
///
/// This is used to tie the lifetime `'a` of a [`WnfDataAccessor<'a, T>`] to the scope of a call to the listener.
/// This is not to be confused with [`WnfDataScope`].
struct WnfScopedData<T>
where
    T: ?Sized,
{
    buffer: *const c_void,
    buffer_size: usize,
    change_stamp: WnfChangeStamp,
    _marker: PhantomData<fn() -> T>,
}

// SAFETY:
// The `buffer` pointer is only used for reading data, which is safe to do from any thread
unsafe impl<T> Send for WnfScopedData<T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Copy`
impl<T> Copy for WnfScopedData<T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Clone`
impl<T> Clone for WnfScopedData<T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Debug`
impl<T> Debug for WnfScopedData<T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WnfScopedData")
            .field("buffer", &self.buffer)
            .field("buffer_size", &self.buffer_size)
            .field("change_stamp", &self.change_stamp)
            .finish()
    }
}

impl<T> WnfScopedData<T>
where
    T: ?Sized,
{
    /// Creates a new [`WnfScopedData<T>`] with the given `buffer`, `buffer_size` and `change_stamp`.
    ///
    /// # Safety
    /// As long as the instance of [`WnfScopedData<T>`] is live:
    /// - `buffer` must be valid for reads of size `buffer_size`
    /// - the memory range of size `buffer_size` starting at `buffer` must be initialized
    unsafe fn new(buffer: *const c_void, buffer_size: usize, change_stamp: WnfChangeStamp) -> Self {
        Self {
            buffer,
            buffer_size,
            change_stamp,
            _marker: PhantomData,
        }
    }

    /// Obtains a [`WnfDataAccessor<'a, T>`] for this [`WnfScopedData<T>`].
    ///
    /// The lifetime parameter `'a` of the returned [`WnfDataAccessor<'a, T>`] is the lifetime of the reference to this
    /// [`WnfScopedData<T>`], making sure the [`WnfDataAccessor<'a, T>`] can only be used as long as this
    /// [`WnfDataScope<T>`] is live.
    fn accessor(&self) -> WnfDataAccessor<T> {
        WnfDataAccessor {
            data: *self,
            _marker: PhantomData,
        }
    }

    /// Casts the data type of this [`WnfScopedData<T>`] to a different type `U`
    ///
    /// The returned [`WnfScopedData<U>`] represents the same underlying data, but treats them as being of a different
    /// type `U`.
    fn cast<U>(self) -> WnfScopedData<U> {
        WnfScopedData {
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
    /// Casts the data type of this [`WnfDataAccessor<'a, T>`] to a different type `U`
    ///
    /// The returned [`WnfDataAccessor<'a, U>`] represents the same underlying data, but treats them as being of a
    /// different type `U`.
    pub fn cast<U>(self) -> WnfDataAccessor<'a, U> {
        WnfDataAccessor {
            data: self.data.cast(),
            _marker: PhantomData,
        }
    }

    /// Queries the change stamp of this [`WnfDataAccessor<'a, T>`]
    ///
    /// The change stamp returned by this method is the change stamp of the underlying WNF state for the update that
    /// caused the listener call to which this [`WnfDataAccessor<'a, T>`] was passed. Note that in contrast to
    /// [`OwnedWnfState::change_stamp`] or [`BorrowedWnfState::change_stamp`], this does not involve an OS call.
    pub fn change_stamp(self) -> WnfChangeStamp {
        self.data.change_stamp
    }
}

impl<T> WnfDataAccessor<'_, T>
where
    T: WnfRead<T>,
{
    /// Queries the data of this [`WnfDataAccessor<'a, T>`]
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`get_boxed`](WnfDataAccessor::get_boxed) method.
    ///
    /// This returns the data of the accessor without a change stamp. In order to query both the data and the change
    /// stamp, use the [`query`](WnfDataAccessor::query) method.
    ///
    /// The data returned by this method is the data of the underlying WNF state for the update that caused the listener
    /// call to which this [`WnfDataAccessor<'a, T>`] was passed. Note that in contrast to [`OwnedWnfState::get`] or
    /// [`BorrowedWnfState::get`], this does not involve an OS call.
    pub fn get(self) -> io::Result<T> {
        self.get_as()
    }

    /// Queries the data of this [`WnfDataAccessor<'a, T>`] together with its change stamp
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`query_boxed`](WnfDataAccessor::query_boxed) method.
    ///
    /// This returns the data of the accessor together with its change stamp as a [`WnfStampedData<T>`]. In order to
    /// only query the data, use the [`get`](WnfDataAccessor::get) method.
    ///
    /// The data returned by this method is the data of the underlying WNF state for the update that caused the listener
    /// call to which this [`WnfDataAccessor<'a, T>`] was passed. Note that in contrast to [`OwnedWnfState::query`] or
    /// [`BorrowedWnfState::query`], this does not involve an OS call.
    pub fn query(self) -> io::Result<WnfStampedData<T>> {
        self.query_as()
    }
}

impl<T> WnfDataAccessor<'_, T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    /// Queries the data of this [`WnfDataAccessor<'a, T>`] as a box
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`get`](WnfDataAccessor::get) method.
    ///
    /// This returns the data of the accessor without a change stamp. In order to query both the data and the change
    /// stamp, use the [`query_boxed`](WnfDataAccessor::query_boxed) method.
    ///
    /// The data returned by this method is the data of the underlying WNF state for the update that caused the listener
    /// call to which this [`WnfDataAccessor<'a, T>`] was passed. Note that in contrast to [`OwnedWnfState::get_boxed`]
    /// or [`BorrowedWnfState::get_boxed`], this does not involve an OS call.
    pub fn get_boxed(self) -> io::Result<Box<T>> {
        self.get_as()
    }

    /// Queries the data of this [`WnfDataAccessor<'a, T>`] as a box together with its change stamp
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`query`](WnfDataAccessor::query) method.
    ///
    /// This returns the data of the accessor together with its change stamp as a [`WnfStampedData<Box<T>>`]. In order
    /// to only query the data, use the [`get_boxed`](OwnedWnfState::get_boxed) method.
    ///
    /// The data returned by this method is the data of the underlying WNF state for the update that caused the listener
    /// call to which this [`WnfDataAccessor<'a, T>`] was passed. Note that in contrast to
    /// [`OwnedWnfState::query_boxed`] or [`BorrowedWnfState::query_boxed`], this does not involve an OS call.
    pub fn query_boxed(self) -> io::Result<WnfStampedData<Box<T>>> {
        self.query_as()
    }
}

impl<T> WnfDataAccessor<'_, T>
where
    T: ?Sized,
{
    /// Queries the data of this [`WnfDataAccessor<'a, T>`] as a value of type `D` without a change stamp
    ///
    /// If `T: Sized`, then `D` can be either `T` or `Box<T>`.
    /// If `T: !Sized`, then `D` must be `Box<T>`.
    pub(crate) fn get_as<D>(self) -> io::Result<D>
    where
        T: WnfRead<D>,
    {
        // SAFETY:
        // - `self` was obtained from a `WnfScopedData<T>` through `WnfScopedData::accessor`, which ties the lifetime
        //   parameter `'a` of `WnfDataAccessor<'a, T>` to the lifetime of the `WnfScopedData<T>`, so the
        //   `WnfScopedData<T>` is still live
        // - `self.data` is a copy of this `WnfScopedData<T>`, which was created through `WnfScopedData::new`
        // - The safety conditions of `WnfScopedData::new` then imply those of `T::from_buffer`
        unsafe { T::from_buffer(self.data.buffer, self.data.buffer_size) }
    }

    /// Queries the data of this [`WnfDataAccessor<'a, T>`] as a value of type `D` together with its change stamp
    ///
    /// If `T: Sized`, then `D` can be either `T` or `Box<T>`.
    /// If `T: !Sized`, then `D` must be `Box<T>`.
    pub(crate) fn query_as<D>(self) -> io::Result<WnfStampedData<D>>
    where
        T: WnfRead<D>,
    {
        Ok(WnfStampedData::from_data_change_stamp(
            self.get_as()?,
            self.data.change_stamp,
        ))
    }
}

/// A subscription of a listener to updates of a WNF state
///
/// This is returned from [`OwnedWnfState::subscribe`] and [`BorrowedWnfState::subscribe`].
///
/// Note that the listener is automatically unsubscribed when the [`WnfSubscription<'a, F>`] is dropped. In
/// this case, errors while unsubscribing are silently ignored. If you want to handle them explicitly, use the
/// [`WnfSubscription::unsubscribe`] method, which returns an [`io::Result<()>`]. Note that the listener will not be
/// called anymore after unsubscribing, even when there is an error.
///
/// If you want to keep the subscription for as long as the process is running and the WNF state exists, use the
/// [`WnfSubscription::forget`] method.
#[must_use = "a `WnfSubscription` is unsubscribed immediately if it is not used"]
pub struct WnfSubscription<'a, F> {
    inner: Option<WnfSubscriptionInner<F>>,
    _marker: PhantomData<&'a ()>,
}

impl<F> WnfSubscription<'_, F> {
    /// Forgets this [`WnfSubscription<'_, F>`], effectively keeping it forever
    ///
    /// When a [`WnfSubscription<'a, F>`] is dropped, the listener is unsubscribed. You can avoid this behavior by
    /// calling this method. It consumes the [`WnfSubscription<'a, F>`] without dropping it, effectively keeping the
    /// subscription for as long as the process is running and the WNF state exists.
    pub fn forget(self) {
        mem::forget(self);
    }

    /// Unsubscribes the listener for thie [`WnfSubscription<'a, F>`]
    ///
    /// This happens automatically when the [`WnfSubscription<'a, F>`] is dropped (unless you call
    /// [`WnfSubscription::forget`]), so there is usually no need to call this method. Its only purpose is to enable you
    /// to handle errors while unsubscribing. Note that the listener will not be called anymore after unsubscribing,
    /// even when there is an error.
    pub fn unsubscribe(mut self) -> io::Result<()> {
        self.try_unsubscribe()
    }

    /// Creates a new [`WnfSubscription<'a, F>`] from the given context and subscription handle
    ///
    /// Note that the lifetime `'a` is inferred at the call site.
    fn new(context: Box<WnfSubscriptionContext<F>>, subscription_handle: *mut c_void) -> Self {
        Self {
            inner: Some(WnfSubscriptionInner {
                context: ManuallyDrop::new(context),
                subscription_handle,
            }),
            _marker: PhantomData,
        }
    }

    fn try_unsubscribe(&mut self) -> io::Result<()> {
        if let Some(inner) = self.inner.take() {
            // SAFETY:
            // - `inner.subscription_handle` was returned from a successful call to
            //   `RtlSubscribeWnfStateChangeNotification`
            // - `RtlUnsubscribeWnfStateChangeNotification` has not been called with `inner.subscription_handle` before
            //    because it is only held in `inner` and `inner` is dropped afterwards
            let result = unsafe { ntapi::RtlUnsubscribeWnfStateChangeNotification(inner.subscription_handle) };

            debug!(
                target: ntapi::TRACING_TARGET,
                ?result,
                input.subscription_handle = inner.subscription_handle as u64,
                "RtlUnsubscribeWnfStateChangeNotification",
            );

            if result.is_ok() {
                ManuallyDrop::into_inner(inner.context);
            } else {
                // In case of an error, we do not call `ManuallyDrop::into_inner`, leaking the
                // `Box<WnfSubscriptionContext<F>>`
                inner.context.clear();
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

// We cannot derive this because that would impose an unnecessary trait bound `F: Debug`
impl<F> Debug for WnfSubscription<'_, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WnfSubscription")
            .field(
                "subscription_handle",
                &self.inner.as_ref().map(|inner| inner.subscription_handle),
            )
            .finish()
    }
}

/// The inner value of a [`WnfSubscription<'a, F>`]
///
/// Unlike [`WnfSubscription<'a, F>`], this does not have a lifetime and is not optional.
struct WnfSubscriptionInner<F> {
    context: ManuallyDrop<Box<WnfSubscriptionContext<F>>>,
    subscription_handle: *mut c_void,
}

// SAFETY:
// By the assumptions on `RtlUnsubscribeWnfStateChangeNotification`, it is safe to call it with a `subscription_handle`
// originating from a different thread
unsafe impl<F> Send for WnfSubscriptionInner<F> where F: Send {}

// We cannot derive this because that would impose an unnecessary trait bound `F: Debug`
impl<F> Debug for WnfSubscriptionInner<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WnfSubscriptionInner")
            .field("subscription_handle", &self.subscription_handle)
            .finish()
    }
}

/// The context of a WNF subscription
///
/// This will be leaked on the heap in case unsubscribing fails.
///
/// We put the listener behind a mutex for two reasons:
/// 1) to avoid race conditions between the subscription callback calling the listener and dropping the listener after
///    (successfully or unsuccessfully) trying to unsubscribe
/// 2) to avoid race conditions between parallel runs of the subscription callback calling the listener
///
/// Note that case 2) does not actually happen in practice because the WNF API runs all listener within a process
/// sequentially on a single thread. However, we don't have to assume this because we need the mutex for case 1) anyway.
struct WnfSubscriptionContext<F>(Mutex<Option<F>>);

// We cannot derive this because that would impose an unnecessary trait bound `F: Debug`
impl<F> Debug for WnfSubscriptionContext<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WnfSubscriptionContext").field(&"..").finish()
    }
}

impl<F> WnfSubscriptionContext<F> {
    /// Creates a new context from the given listener
    fn new(listener: F) -> Self {
        Self(Mutex::new(Some(listener)))
    }

    /// Clears the context
    ///
    /// This removes the listener from the context, causing it to be dropped and not be called anymore. This is useful
    /// when unsubscribing fails and we need to leak the context but still want to drop the listener itself.
    fn clear(&self) {
        // We can access the `Option<F>` even when the mutex is poisoned as we're only overwriting it with `None` and
        // hence have no invariant to maintain
        let mut listener = match self.0.lock() {
            Ok(context) => context,
            Err(err) => err.into_inner(),
        };

        *listener = None;
    }

    /// Calls the given closure on the listener contained in this context, if any
    fn with_listener(&self, op: impl FnOnce(&mut F)) {
        if let Ok(mut listener) = self.0.lock() {
            if let Some(listener) = listener.as_mut() {
                op(listener);
            }
        }
    }
}
