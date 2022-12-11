//! Methods for subscribing to state changes

use std::ffi::c_void;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::sync::Mutex;
use std::{fmt, io, mem, panic, ptr};

use tracing::{debug, trace_span};
use windows::core::GUID;
use windows::Win32::Foundation::{NTSTATUS, STATUS_SUCCESS};

use crate::data::{ChangeStamp, StampedData};
use crate::ntapi;
use crate::read::Read;
use crate::state::{BorrowedState, OwnedState, RawState};
use crate::state_name::StateName;

/// The change stamp that a state listener has last seen
///
/// The [`OwnedState::subscribe`] and [`BorrowedState::subscribe`] methods expect an argument of this type to
/// indicate what change stamp of the state the listener has last seen. The listener will then only be notified about
/// updates with a (strictly) larger change stamp, i.e. updates that happen after the one it has last seen.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum SeenChangeStamp {
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
    Value(ChangeStamp),
}

/// Types capable of listening to state updates
///
/// Note that there is a blanket implementation of this trait for all closure types
/// `F: FnMut(DataAccessor<T>)`, so you usually don't need to implement this trait for your own types. It is useful,
/// however, if you need a state listener whose type you can name explicitly. Since closure types are anonymous, you
/// can instead define your own type and implement [`StateListener<T>`] for it.
pub trait StateListener<T>
where
    T: ?Sized,
{
    /// Calls this state listener
    ///
    /// The provided [`DataAccessor<T>`] can be used to obtain the state data at the time the update took place.
    fn call(&mut self, accessor: DataAccessor<'_, T>);
}

impl<F, T> StateListener<T> for F
where
    F: FnMut(DataAccessor<'_, T>),
    T: ?Sized,
{
    fn call(&mut self, accessor: DataAccessor<'_, T>) {
        self(accessor);
    }
}

impl<T> OwnedState<T>
where
    T: ?Sized,
{
    /// Subscribes the given state listener to this state
    ///
    /// The `last_seen_change_stamp` argument can be used to indicate what change stamp of the state you have last seen,
    /// which has an impact on which state updates the listener is notified about. See [`SeenChangeStamp`] for the
    /// available options.
    ///
    /// Note that the listener is automatically unsubscribed when the returned [`Subscription<'a, F>`] is dropped. In
    /// this case, errors while unsubscribing are silently ignored. If you want to handle them explicitly, use the
    /// [`Subscription::unsubscribe`] method, which returns an [`io::Result<()>`].
    ///
    /// In any case, the listener will not be called anymore after unsubscribing, even when there is an error. However,
    /// in order to maintain memory safety, in the case of an error a value the size of a [`Mutex<Option<F>>`] is leaked
    /// on the heap. This should be fine in most cases, especially when `F` is small. Otherwise consider using a boxed
    /// closure.
    ///
    /// # Example
    ///
    /// ```
    /// use wnf::{DataAccessor, OwnedState, SeenChangeStamp};
    ///
    /// let state = OwnedState::create_temporary().unwrap();
    /// state.set(&0).expect("failed to set state data");
    ///
    /// let _subscripton = state
    ///     .subscribe(
    ///         |accessor: DataAccessor<_>| {
    ///             let value = accessor.get().expect("failed to get state data");
    ///             println!("State data updated: {value}");
    ///         },
    ///         SeenChangeStamp::Current,
    ///     )
    ///     .expect("failed to subscribe to state updates");
    ///
    /// state.set(&1).expect("failed to set state data");
    /// ```
    ///
    /// This prints:
    /// ```no_compile
    /// State data updated: 1
    /// ```
    ///
    /// # Errors
    /// Returns an error if subscribing fails
    pub fn subscribe<F>(&self, listener: F, last_seen_change_stamp: SeenChangeStamp) -> io::Result<Subscription<'_, F>>
    where
        F: StateListener<T> + Send + 'static,
    {
        self.raw.subscribe(listener, last_seen_change_stamp)
    }
}

impl<'a, T> BorrowedState<'a, T>
where
    T: ?Sized,
{
    /// Subscribes the given state listener to this state
    ///
    /// See [`OwnedState::subscribe`]
    pub fn subscribe<F>(self, listener: F, last_seen_change_stamp: SeenChangeStamp) -> io::Result<Subscription<'a, F>>
    where
        F: StateListener<T> + Send + 'static,
    {
        self.raw.subscribe(listener, last_seen_change_stamp)
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    /// Subscribes the given state listener to this state
    pub(crate) fn subscribe<'a, F>(
        &self,
        listener: F,
        last_seen_change_stamp: SeenChangeStamp,
    ) -> io::Result<Subscription<'a, F>>
    where
        F: StateListener<T> + Send + 'static,
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
            F: StateListener<T> + Send + 'static,
            T: ?Sized,
        {
            let _ = panic::catch_unwind(|| {
                let span = trace_span!(
                    target: ntapi::TRACING_TARGET,
                    "WnfUserCallback",
                    input.state_name = %StateName::from_opaque_value(state_name),
                    input.change_stamp = change_stamp,
                    input.buffer_size = buffer_size
                );
                let _enter = span.enter();

                // SAFETY:
                // (1) By the assumption on `RtlSubscribeWnfStateChangeNotification`, `context` is the pointer passed in
                // the fifth argument of some successful call to that function. Let `subscription_handle` be the
                // subscription handle returned from that call. Then `context` points to a `SubscriptionContext<F>`
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
                // From (1), it follows that the `SubscriptionContext<F>` was not dropped in the `subscribe` method
                // but returned wrapped in a `Subscription<'a, F>`.
                //
                // In case (a) we know that the `Subscription<'a, F>` has not been dropped because dropping it would
                // have called `RtlUnsubscribeWnfStateChangeNotification` with `subscription_handle`. Hence the
                // `SubscriptionContext<F>` hasn't been dropped either.
                //
                // In case (b) the `Subscription<'a, F>` has been dropped but the `SubscriptionContext<F>` it
                // contains has been leaked (see comment below).
                //
                // In any case, `context` points to a valid `SubscriptionContext<F>`.
                //
                // (3) We may be on a different thread than the one that created the `SubscriptionContext<F>`, but
                // `F: Send` implies `SubscriptionContext<F>: Send`.
                //
                // (4) `F` outlives the lifetime of the produced reference because `F: 'static`.
                let context: &SubscriptionContext<F> = unsafe { &*context.cast() };

                // SAFETY:
                // - By the assumption on `RtlSubscribeWnfStateChangeNotification`, the assumption on `WnfUserCallback`
                //   is satisfied
                // - As `data` is dropped before `callback` returns, the assumption on `WnfUserCallback` then implies
                //   the safety conditions of `ScopedData::new`
                let data = unsafe { ScopedData::new(buffer, buffer_size as usize, change_stamp) };

                context.with_listener(|listener| {
                    listener.call(data.accessor());
                });
            });

            STATUS_SUCCESS
        }

        let change_stamp = match last_seen_change_stamp {
            SeenChangeStamp::None => ChangeStamp::initial(),
            SeenChangeStamp::Current => self.change_stamp()?,
            SeenChangeStamp::Value(value) => value,
        };

        let mut subscription_handle = SubscriptionHandle::null();
        let context = Box::new(SubscriptionContext::new(listener));

        // SAFETY:
        // - The pointer in the first argument is valid for writes of `*mut c_void` because it comes from a live mutable
        //   reference to a `SubscriptionHandle`, which is a #[repr(transparent)] wrapper around `*mut c_void`
        // - The function pointed to by the pointer in the fourth argument does not unwind because it catches all
        //   unwinding panics
        // - The pointer in the fifth argument is either a null pointer or points to a valid `GUID` by the guarantees of
        //   `TypeId::as_ptr`
        let result = unsafe {
            ntapi::RtlSubscribeWnfStateChangeNotification(
                &mut subscription_handle as *mut SubscriptionHandle as *mut *mut c_void,
                self.state_name.opaque_value(),
                change_stamp.into(),
                callback::<F, T>,
                &*context as *const SubscriptionContext<F> as *mut c_void,
                self.type_id.as_ptr(),
                0,
                0,
            )
        };

        if result.is_ok() {
            let subscription = Subscription::new(context, subscription_handle);

            debug!(
                target: ntapi::TRACING_TARGET,
                ?result,
                input.state_name = %self.state_name,
                input.change_stamp = %change_stamp,
                input.type_id = %self.type_id,
                output.subscription_handle = %subscription_handle,
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
/// Listeners receive a [`DataAccessor<'a, T>`] in their [`StateListener::call`] method. It can be used to obtain
/// the state data at the time the update took place.
///
/// The lifetime parameter `'a` ties a [`DataAccessor<'a, T>`] to the lifetime of the state data, which is only valid
/// within the scope of the call to the listener.
pub struct DataAccessor<'a, T>
where
    T: ?Sized,
{
    data: ScopedData,
    _marker: PhantomData<&'a fn() -> T>,
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Copy`
impl<T> Copy for DataAccessor<'_, T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Clone`
impl<T> Clone for DataAccessor<'_, T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Debug`
impl<T> Debug for DataAccessor<'_, T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataAccessor").field("data", &self.data).finish()
    }
}

/// State data that is only valid within a certain scope
///
/// This is used to tie the lifetime `'a` of a [`DataAccessor<'a, T>`] to the scope of a call to the listener.
/// This is not to be confused with [`DataScope`].
#[derive(Clone, Copy, Debug)]
struct ScopedData {
    buffer: *const c_void,
    buffer_size: usize,
    change_stamp: ChangeStamp,
}

// SAFETY:
// The `buffer` pointer is only used for reading data
unsafe impl Send for ScopedData {}

// SAFETY:
// The `buffer` pointer is only used for reading data
unsafe impl Sync for ScopedData {}

impl ScopedData {
    /// Creates a new [`ScopedData`] with the given `buffer`, `buffer_size` and `change_stamp`
    ///
    /// # Safety
    /// As long as the instance of [`ScopedData`] is live:
    /// - `buffer` must be valid for reads of size `buffer_size`
    /// - the memory range of size `buffer_size` starting at `buffer` must be initialized
    unsafe fn new(buffer: *const c_void, buffer_size: usize, change_stamp: impl Into<ChangeStamp>) -> Self {
        Self {
            buffer,
            buffer_size,
            change_stamp: change_stamp.into(),
        }
    }

    /// Obtains a [`DataAccessor<'a, T>`] for this [`ScopedData`]
    ///
    /// The lifetime parameter `'a` of the returned [`DataAccessor<'a, T>`] is the lifetime of the reference to this
    /// [`ScopedData`], making sure the [`DataAccessor<'a, T>`] can only be used as long as this [`ScopedData`] is live.
    const fn accessor<T>(&self) -> DataAccessor<'_, T>
    where
        T: ?Sized,
    {
        DataAccessor {
            data: *self,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> DataAccessor<'a, T>
where
    T: ?Sized,
{
    /// Casts the data type of this [`DataAccessor<'a, T>`] to a different type `U`
    ///
    /// The returned [`DataAccessor<'a, U>`] represents the same underlying data, but treats them as being of a
    /// different type `U`.
    pub const fn cast<U>(self) -> DataAccessor<'a, U> {
        DataAccessor {
            data: self.data,
            _marker: PhantomData,
        }
    }

    /// Queries the change stamp of this [`DataAccessor<'a, T>`]
    ///
    /// The change stamp returned by this method is the change stamp of the underlying state for the update that
    /// caused the listener call to which this [`DataAccessor<'a, T>`] was passed. Note that in contrast to
    /// [`OwnedState::change_stamp`] or [`BorrowedState::change_stamp`], this does not involve an OS call.
    pub const fn change_stamp(self) -> ChangeStamp {
        self.data.change_stamp
    }
}

impl<T> DataAccessor<'_, T>
where
    T: Read<T>,
{
    /// Queries the data of this [`DataAccessor<'a, T>`]
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`get_boxed`](DataAccessor::get_boxed) method.
    ///
    /// This returns the data of the accessor without a change stamp. In order to query both the data and the change
    /// stamp, use the [`query`](DataAccessor::query) method.
    ///
    /// The data returned by this method is the data of the underlying state for the update that caused the listener
    /// call to which this [`DataAccessor<'a, T>`] was passed. Note that in contrast to [`OwnedState::get`] or
    /// [`BorrowedState::get`], this does not involve an OS call.
    ///
    /// # Errors
    /// Returns an error if querying fails, including the case that the queried data is not a valid `T`
    pub fn get(self) -> io::Result<T> {
        self.get_as()
    }

    /// Queries the data of this [`DataAccessor<'a, T>`] together with its change stamp
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`query_boxed`](DataAccessor::query_boxed) method.
    ///
    /// This returns the data of the accessor together with its change stamp as a [`StampedData<T>`]. In order to
    /// only query the data, use the [`get`](DataAccessor::get) method.
    ///
    /// The data returned by this method is the data of the underlying state for the update that caused the listener
    /// call to which this [`DataAccessor<'a, T>`] was passed. Note that in contrast to [`OwnedState::query`] or
    /// [`BorrowedState::query`], this does not involve an OS call.
    ///
    /// # Errors
    /// Returns an error if querying fails, including the case that the queried data is not a valid `T`
    pub fn query(self) -> io::Result<StampedData<T>> {
        self.query_as()
    }
}

impl<T> DataAccessor<'_, T>
where
    T: Read<Box<T>> + ?Sized,
{
    /// Queries the data of this [`DataAccessor<'a, T>`] as a box
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`get`](DataAccessor::get) method.
    ///
    /// This returns the data of the accessor without a change stamp. In order to query both the data and the change
    /// stamp, use the [`query_boxed`](DataAccessor::query_boxed) method.
    ///
    /// The data returned by this method is the data of the underlying state for the update that caused the listener
    /// call to which this [`DataAccessor<'a, T>`] was passed. Note that in contrast to [`OwnedState::get_boxed`]
    /// or [`BorrowedState::get_boxed`], this does not involve an OS call.
    ///
    /// # Errors
    /// Returns an error if querying fails, including the case that the queried data is not a valid `T`
    pub fn get_boxed(self) -> io::Result<Box<T>> {
        self.get_as()
    }

    /// Queries the data of this [`DataAccessor<'a, T>`] as a box together with its change stamp
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`query`](DataAccessor::query) method.
    ///
    /// This returns the data of the accessor together with its change stamp as a [`StampedData<Box<T>>`]. In order
    /// to only query the data, use the [`get_boxed`](OwnedState::get_boxed) method.
    ///
    /// The data returned by this method is the data of the underlying state for the update that caused the listener
    /// call to which this [`DataAccessor<'a, T>`] was passed. Note that in contrast to
    /// [`OwnedState::query_boxed`] or [`BorrowedState::query_boxed`], this does not involve an OS call.
    ///
    /// # Errors
    /// Returns an error if querying fails, including the case that the queried data is not a valid `T`
    pub fn query_boxed(self) -> io::Result<StampedData<Box<T>>> {
        self.query_as()
    }
}

impl<T> DataAccessor<'_, T>
where
    T: ?Sized,
{
    /// Queries the data of this [`DataAccessor<'a, T>`] as a value of type `D` without a change stamp
    ///
    /// If `T: Sized`, then `D` can be either `T` or `Box<T>`.
    /// If `T: !Sized`, then `D` must be `Box<T>`.
    pub(crate) fn get_as<D>(self) -> io::Result<D>
    where
        T: Read<D>,
    {
        // SAFETY:
        // - `self` was obtained from a `ScopedData` through `ScopedData::accessor`, which ties the lifetime parameter
        //   `'a` of `DataAccessor<'a, T>` to the lifetime of the `ScopedData`, so the `ScopedData` is still live
        // - `self.data` is a copy of this `ScopedData`, which was created through `ScopedData::new`
        // - The safety conditions of `ScopedData::new` then imply those of `T::from_buffer`
        unsafe { T::from_buffer(self.data.buffer, self.data.buffer_size) }
    }

    /// Queries the data of this [`DataAccessor<'a, T>`] as a value of type `D` together with its change stamp
    ///
    /// If `T: Sized`, then `D` can be either `T` or `Box<T>`.
    /// If `T: !Sized`, then `D` must be `Box<T>`.
    pub(crate) fn query_as<D>(self) -> io::Result<StampedData<D>>
    where
        T: Read<D>,
    {
        Ok(StampedData::from_data_change_stamp(
            self.get_as()?,
            self.data.change_stamp,
        ))
    }
}

/// A subscription of a listener to updates of a state
///
/// This is returned from [`OwnedState::subscribe`] and [`BorrowedState::subscribe`].
///
/// Note that the listener is automatically unsubscribed when the [`Subscription<'a, F>`] is dropped. In
/// this case, errors while unsubscribing are silently ignored. If you want to handle them explicitly, use the
/// [`Subscription::unsubscribe`] method, which returns an [`io::Result<()>`]. Note that the listener will not be
/// called anymore after unsubscribing, even when there is an error.
///
/// If you want to keep the subscription for as long as the process is running and the state exists, use the
/// [`Subscription::forget`] method.
#[must_use = "a `Subscription` is unsubscribed immediately if it is not used"]
pub struct Subscription<'a, F> {
    inner: Option<SubscriptionInner<F>>,
    _marker: PhantomData<&'a ()>,
}

impl<F> Subscription<'_, F> {
    /// Forgets this [`Subscription<'_, F>`], effectively keeping it forever
    ///
    /// When a [`Subscription<'a, F>`] is dropped, the listener is unsubscribed. You can avoid this behavior by
    /// calling this method. It consumes the [`Subscription<'a, F>`] without dropping it, effectively keeping the
    /// subscription for as long as the process is running and the state exists.
    pub const fn forget(self) {
        mem::forget(self);
    }

    /// Unsubscribes the listener for thie [`Subscription<'a, F>`]
    ///
    /// This happens automatically when the [`Subscription<'a, F>`] is dropped (unless you call
    /// [`Subscription::forget`]), so there is usually no need to call this method. Its only purpose is to enable you
    /// to handle errors while unsubscribing. Note that the listener will not be called anymore after unsubscribing,
    /// even when there is an error.
    ///
    /// # Errors
    /// Returns an error if unsubscribing fails
    pub fn unsubscribe(mut self) -> io::Result<()> {
        self.try_unsubscribe()
    }

    /// Creates a new [`Subscription<'a, F>`] from the given context and subscription handle
    ///
    /// Note that the lifetime `'a` is inferred at the call site.
    const fn new(context: Box<SubscriptionContext<F>>, subscription_handle: SubscriptionHandle) -> Self {
        Self {
            inner: Some(SubscriptionInner {
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
            //   because it is only held in `inner` and `inner` is dropped afterwards
            let result = unsafe { ntapi::RtlUnsubscribeWnfStateChangeNotification(inner.subscription_handle.as_ptr()) };

            debug!(
                target: ntapi::TRACING_TARGET,
                ?result,
                input.subscription_handle = %inner.subscription_handle,
                "RtlUnsubscribeWnfStateChangeNotification",
            );

            if result.is_ok() {
                ManuallyDrop::into_inner(inner.context);
            } else {
                // In case of an error, we do not call `ManuallyDrop::into_inner`, leaking the
                // `Box<SubscriptionContext<F>>`
                inner.context.clear();
            }

            result.ok()?;
        };

        Ok(())
    }
}

impl<F> Drop for Subscription<'_, F> {
    fn drop(&mut self) {
        let _ = self.try_unsubscribe();
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `F: Debug`
impl<F> Debug for Subscription<'_, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Subscription")
            .field(
                "subscription_handle",
                &self.inner.as_ref().map(|inner| inner.subscription_handle),
            )
            .finish()
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `F: PartialEq<F>`
impl<F> PartialEq for Subscription<'_, F> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `F: Eq`
impl<F> Eq for Subscription<'_, F> {}

// We cannot derive this because that would impose an unnecessary trait bound `F: Hash`
impl<F> Hash for Subscription<'_, F> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

/// The inner value of a [`Subscription<'a, F>`]
///
/// Unlike [`Subscription<'a, F>`], this does not have a lifetime and is not optional.
struct SubscriptionInner<F> {
    context: ManuallyDrop<Box<SubscriptionContext<F>>>,
    subscription_handle: SubscriptionHandle,
}

// We cannot derive this because that would impose an unnecessary trait bound `F: Debug`
impl<F> Debug for SubscriptionInner<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubscriptionInner")
            .field("subscription_handle", &self.subscription_handle)
            .finish()
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `F: PartialEq<F>`
impl<F> PartialEq for SubscriptionInner<F> {
    fn eq(&self, other: &Self) -> bool {
        self.subscription_handle == other.subscription_handle
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `F: Eq`
impl<F> Eq for SubscriptionInner<F> {}

impl<F> Hash for SubscriptionInner<F> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.subscription_handle.hash(state);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
struct SubscriptionHandle(*mut c_void);

// SAFETY:
// By the assumptions on `RtlUnsubscribeWnfStateChangeNotification`, it is safe to call it with a subscription handle
// originating from a different thread
unsafe impl Send for SubscriptionHandle {}

// SAFETY:
// By the assumptions on `RtlUnsubscribeWnfStateChangeNotification`, it is safe to call it with a subscription handle
// originating from a different thread
unsafe impl Sync for SubscriptionHandle {}

impl SubscriptionHandle {
    /// Creates a NULL [`SubscriptionHandle`]
    const fn null() -> Self {
        Self(ptr::null_mut())
    }

    /// Returns a mutable raw pointer to the inner value for use in FFI
    const fn as_ptr(&self) -> *mut c_void {
        self.0
    }
}

impl Display for SubscriptionHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.0 as u64)
    }
}

/// The context of a subscription
///
/// This will be leaked on the heap in case unsubscribing fails.
///
/// We put the listener behind a mutex for two reasons:
/// 1) to avoid race conditions between the subscription callback calling the listener and dropping the listener after
///    (successfully or unsuccessfully) trying to unsubscribe
/// 2) to avoid race conditions between parallel runs of the subscription callback calling the listener
///
/// Note that case 2) does not actually happen in practice because the WNF API runs all listeners within a process
/// sequentially on a single thread. However, we don't have to assume this because we need the mutex for case 1) anyway.
struct SubscriptionContext<F>(Mutex<Option<F>>);

// We cannot derive this because that would impose an unnecessary trait bound `F: Debug`
impl<F> Debug for SubscriptionContext<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SubscriptionContext").field(&"..").finish()
    }
}

impl<F> SubscriptionContext<F> {
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

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    use std::cell::Cell;

    use static_assertions::{assert_impl_all, assert_not_impl_any};

    use super::*;

    #[test]
    fn subscription_handle_display() {
        assert_eq!(SubscriptionHandle::null().to_string(), "0x0000000000000000");
    }

    #[test]
    fn data_accessor_is_send_and_sync_regardless_of_data_type() {
        type NeitherSendNorSync = *const ();
        assert_not_impl_any!(NeitherSendNorSync: Send, Sync);

        assert_impl_all!(DataAccessor<'_, NeitherSendNorSync>: Send, Sync);
    }

    #[test]
    fn subscription_is_send_and_sync_if_listener_is_send() {
        type SendNotSync = Cell<()>;
        assert_impl_all!(SendNotSync: Send);
        assert_not_impl_any!(SendNotSync: Sync);

        assert_impl_all!(Subscription<'_, ()>: Send, Sync);
    }
}
