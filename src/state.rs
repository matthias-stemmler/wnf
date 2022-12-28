//! Types representing states
//!
//! Additional `impl` blocks for these types can be found in other modules of this crate.

#![deny(unsafe_code)]

use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::Deref;

use crate::state_name::StateName;
use crate::type_id::{TypeId, GUID};

/// An owned state
///
/// This deletes the represented state on drop. You can prevent this behavior by calling the
/// [`leak`](OwnedState::leak) method.
///
/// While ownership in Rust usually refers to the ownership of memory, this applies the idea of ownership to an
/// external entity, namely a state. It's similar to [`OwnedHandle`](std::os::windows::io::OwnedHandle) in that
/// regard.
///
/// An [`OwnedState<T>`] can be borrowed as a [`BorrowedState<'_, T>`](BorrowedState) using the [`AsState::as_state`]
/// method.
///
/// The type parameter `T` is the type of data contained in the state.
pub struct OwnedState<T>
where
    T: ?Sized,
{
    pub(crate) raw: RawState<T>,
}

impl<T> OwnedState<T>
where
    T: ?Sized,
{
    /// Returns the name of this state
    pub const fn state_name(&self) -> StateName {
        self.raw.state_name()
    }

    /// Leaks this [`OwnedState<T>`]
    ///
    /// This consumes the [`OwnedState<T>`] without dropping it, returning a [`BorrowedState<'static,
    /// T>`](BorrowedState) with `'static` lifetime that represents the same underlying state.
    ///
    /// Note that while this is named after [`Box::leak`], it doesn't leak memory, but it leaks a state in the sense
    /// that the state doesn't get deleted on drop.
    pub fn leak(self) -> BorrowedState<'static, T> {
        BorrowedState::from_raw(self.into_raw())
    }

    /// Casts the data type of this state to a different type `U`
    ///
    /// The returned [`OwnedState<U>`] represents the same underlying state, but treats it as containing data of
    /// a different type `U`.
    pub fn cast<U>(self) -> OwnedState<U>
    where
        U: ?Sized,
    {
        OwnedState::from_raw(self.into_raw().cast())
    }

    /// Creates a new [`OwnedState`] wrapping a given [`RawState`]
    pub(crate) const fn from_raw(raw: RawState<T>) -> Self {
        Self { raw }
    }

    /// Consumes this [`OwnedState`] without dropping it, returning the inner [`RawState`]
    pub(crate) fn into_raw(self) -> RawState<T> {
        ManuallyDrop::new(self).raw
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: PartialEq<T>`
impl<T> PartialEq for OwnedState<T>
where
    T: ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Eq`
impl<T> Eq for OwnedState<T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Hash`
impl<T> Hash for OwnedState<T>
where
    T: ?Sized,
{
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.raw.hash(state);
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Debug`
impl<T> Debug for OwnedState<T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnedState").field("raw", &self.raw).finish()
    }
}

impl<T> Drop for OwnedState<T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        let _ = self.raw.delete();
    }
}

/// A borrowed state
///
/// This has a lifetime parameter to tie it to something that owns the state, typically an [`OwnedState<T>`].
///
/// Unlike [`OwnedState<T>`], this implements [`Copy`] (and [`Clone`]) and does not delete the represented state on
/// drop.
///
/// While borrowing in Rust usually refers to borrowing memory, this applies the idea of borrowing to an external
/// entity, namely a state. It's similar to [`BorrowedHandle<'_>`](std::os::windows::io::BorrowedHandle) in that
/// regard.
///
/// Calling [`Clone::clone`] on a [`BorrowedState<'a, T>`](BorrowedState) just makes a trivial copy, returning another
/// [`BorrowedState<'a, T>`](BorrowedState) with the same lifetime as the original one and representing the same
/// underlying WNF state. The same applies to the [`ToOwned::to_owned`] method.  If you want to turn a
/// [`BorrowedState<'_, T>`](BorrowedState) into an [`OwnedState<T>`] (which will then delete the underlying state on
/// drop), use the [`BorrowedState::to_owned_state`] method.
///
/// The type parameter `T` is the type of data contained in the state.
pub struct BorrowedState<'a, T>
where
    T: ?Sized,
{
    pub(crate) raw: RawState<T>,
    _marker: PhantomData<&'a ()>,
}

impl<'a, T> BorrowedState<'a, T>
where
    T: ?Sized,
{
    /// Returns the name of this state
    pub const fn state_name(self) -> StateName {
        self.raw.state_name()
    }

    /// Turns this [`BorrowedState<'_, T>`](BorrowedState) into an [`OwnedState<T>`] representing the same underlying
    /// state
    ///
    /// Note that the underlying state will be deleted when the [`OwnedState<T>`] is dropped.
    pub const fn to_owned_state(self) -> OwnedState<T> {
        OwnedState::from_raw(self.raw)
    }

    /// Casts the data type of this state to a different type `U`
    ///
    /// The returned [`BorrowedState<'a, U>`](BorrowedState) represents the same underlying state, but treats it as
    /// containing data of a different type `U`.
    pub const fn cast<U>(self) -> BorrowedState<'a, U>
    where
        U: ?Sized,
    {
        BorrowedState::from_raw(self.raw.cast())
    }

    /// Creates a new [`BorrowedState<'_, T>`](BorrowedState) wrapping a given [`RawState<T>`]
    ///
    /// The lifetime `'a` of the returned [`BorrowedState<'a, T>`](BorrowedState) is inferred at the call site.
    pub(crate) const fn from_raw(raw: RawState<T>) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }
}

impl<T> BorrowedState<'static, T>
where
    T: ?Sized,
{
    /// Statically borrows the state with the given name
    ///
    /// Note that an underlying state with the given name may or may not exist. The returned
    /// [`BorrowedState<'static, T>`](BorrowedState) having a `'static` lifetime just means that the state is borrowed
    /// directly from the system rather than from an [`OwnedState<T>`] that will be dropped at some point.
    pub fn from_state_name(state_name: impl Into<StateName>) -> Self {
        Self::from_raw(RawState::from_state_name_and_type_id(state_name.into(), TypeId::none()))
    }

    /// Statically borrows the state with the given name using the given type id
    ///
    /// Note that an underlying state with the given name may or may not exist. The returned
    /// [`BorrowedState<'static, T>`](BorrowedState) having a `'static` lifetime just means that the state is borrowed
    /// directly from the system rather than from an [`OwnedState<T>`] that will be dropped at some point.
    pub fn from_state_name_and_type_id(state_name: impl Into<StateName>, type_id: impl Into<GUID>) -> Self {
        Self::from_raw(RawState::from_state_name_and_type_id(
            state_name.into(),
            TypeId::from_guid(type_id.into()),
        ))
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Copy`
impl<T> Copy for BorrowedState<'_, T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Clone`
impl<T> Clone for BorrowedState<'_, T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: PartialEq<T>`
impl<T> PartialEq for BorrowedState<'_, T>
where
    T: ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Eq`
impl<T> Eq for BorrowedState<'_, T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Hash`
impl<T> Hash for BorrowedState<'_, T>
where
    T: ?Sized,
{
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.raw.hash(state);
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Debug`
impl<T> Debug for BorrowedState<'_, T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowedState").field("raw", &self.raw).finish()
    }
}

/// A trait for types that can be borrowed as a state
///
/// This is implemented for both [`OwnedState<T>`] and [`BorrowedState<'_, T>`](BorrowedState). There are two main use
/// cases for it:
///
/// - Functions that can accept either a reference to an owned or a borrowed state: Even though a
/// [`BorrowedState<'_, T>`](BorrowedState) plays the role of a reference to a state, it's not technically a reference.
/// As a consequence, there is no deref coercion for states, i.e. you cannot just pass an [`&'a
/// OwnedState<T>`](OwnedState) where a [`BorrowedState<'a, T>`](BorrowedState) is expected. In order to accept both
/// types, functions can instead take a reference to a generic type implementing [`AsState`]:
/// ```
/// # use std::io;
/// # use wnf::{AsState, OwnedState};
/// #
/// fn add_to_state(state: &impl AsState<Data = u32>, delta: u32) -> io::Result<()> {
///     let state = state.as_state();
///     let value = state.get()?;
///     let new_value = value + delta;
///     state.set(&new_value)
/// }
/// ```
///
/// - Types that can contain either an owned or a borrowed state:
/// ```
/// # use std::io;
/// # use wnf::AsState;
/// #
/// struct StateWrapper<S> {
///     state: S,
/// }
///
/// impl<S> StateWrapper<S>
/// where
///     S: AsState<Data = u32>,
/// {
///     fn add(&self, delta: u32) -> io::Result<()> {
///         let state = self.state.as_state();
///         let value = state.get()?;
///         let new_value = value + delta;
///         state.set(&new_value)
///     }
/// }
/// ```
///
/// When comparing [`OwnedState<T>`] with [`OwnedHandle`](std::os::windows::io::OwnedHandle) and
/// [`BorrowedState<'_, T>`](BorrowedState) with [`BorrowedHandle<'_>`](std::os::windows::io::BorrowedHandle), this
/// trait plays the role of the [`AsHandle`](std::os::windows::io::AsHandle) trait.
///
/// This trait is sealed and cannot by implemented outside of the `wnf` crate.
pub trait AsState: private::Sealed {
    /// The type of the data contained in the borrowed state
    type Data: ?Sized;

    /// Borrows a value as a state
    fn as_state(&self) -> BorrowedState<'_, Self::Data>;
}

impl<T> AsState for OwnedState<T>
where
    T: ?Sized,
{
    type Data = T;

    fn as_state(&self) -> BorrowedState<'_, T> {
        BorrowedState::from_raw(self.raw)
    }
}

impl<T> AsState for BorrowedState<'_, T>
where
    T: ?Sized,
{
    type Data = T;

    fn as_state(&self) -> BorrowedState<'_, T> {
        *self
    }
}

impl<S> AsState for S
where
    S: Deref,
    S::Target: AsState,
{
    type Data = <S::Target as AsState>::Data;

    fn as_state(&self) -> BorrowedState<'_, Self::Data> {
        self.deref().as_state()
    }
}

/// A raw state
///
/// This neither deletes the underlying state on drop, nor does it have a lifetime.
pub(crate) struct RawState<T>
where
    T: ?Sized,
{
    pub(crate) state_name: StateName,
    pub(crate) type_id: TypeId,
    // `RawState<T>` is neither covariant nor contravariant in `T` and doesn't own a `T`
    _marker: PhantomData<fn(T) -> T>,
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    /// Creates a [`RawState<T>`] with the given name using the given type id
    pub(crate) const fn from_state_name_and_type_id(state_name: StateName, type_id: TypeId) -> Self {
        Self {
            state_name,
            type_id,
            _marker: PhantomData,
        }
    }

    /// Returns the name of this state
    const fn state_name(self) -> StateName {
        self.state_name
    }

    /// Casts the data type of this state to a different type `U`
    ///
    /// The returned [`RawState<U>`] represents the same underlying state, but treats it as containing data of
    /// a different type `U`.
    pub(crate) const fn cast<U>(self) -> RawState<U>
    where
        U: ?Sized,
    {
        RawState::from_state_name_and_type_id(self.state_name, self.type_id)
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Copy`
impl<T> Copy for RawState<T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Clone`
impl<T> Clone for RawState<T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: PartialEq<T>`
impl<T> PartialEq for RawState<T>
where
    T: ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        self.state_name == other.state_name && self.type_id == other.type_id
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Eq`
impl<T> Eq for RawState<T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Hash`
impl<T> Hash for RawState<T>
where
    T: ?Sized,
{
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.state_name.hash(state);
        self.type_id.hash(state);
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Debug`
impl<T> Debug for RawState<T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawState")
            .field("state_name", &self.state_name)
            .field("type_id", &self.type_id)
            .finish()
    }
}

/// Making [`AsState`] a sealed trait
mod private {
    use std::ops::Deref;

    use super::{BorrowedState, OwnedState};

    pub trait Sealed {}

    impl<T> Sealed for OwnedState<T> where T: ?Sized {}
    impl<T> Sealed for BorrowedState<'_, T> where T: ?Sized {}
    impl<S> Sealed for S
    where
        S: Deref,
        S::Target: Sealed,
    {
    }
}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    use static_assertions::{assert_impl_all, assert_not_impl_any};

    use super::*;

    #[test]
    fn owned_state_is_send_and_sync_regardless_of_data_type() {
        type NeitherSendNorSync = *const ();
        assert_not_impl_any!(NeitherSendNorSync: Send, Sync);

        assert_impl_all!(OwnedState<NeitherSendNorSync>: Send, Sync);
    }

    #[test]
    fn borrowed_state_is_send_and_sync_regardless_of_data_type() {
        type NeitherSendNorSync = *const ();
        assert_not_impl_any!(NeitherSendNorSync: Send, Sync);

        assert_impl_all!(BorrowedState<'_, NeitherSendNorSync>: Send, Sync);
    }
}
