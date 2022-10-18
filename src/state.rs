//! Types representing WNF states
//!
//! Additional `impl` blocks for these types can be found in other modules of this crate

#![deny(unsafe_code)]

use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;

use crate::state_name::WnfStateName;
use crate::type_id::{TypeId, GUID};

/// An owned WNF state
///
/// This deletes the represented WNF state on drop. You can prevent this behavior by calling the
/// [`leak`](OwnedWnfState::leak) method.
///
/// While ownership in Rust usually refers to the ownership of memory, this applies the idea of ownership to an
/// external entity, namely a WNF state. It's similar to [`OwnedHandle`](std::os::windows::io::OwnedHandle) in that
/// regard.
///
/// An [`OwnedWnfState<T>`] can be borrowed as a [`BorrowedWnfState<'a, T>`] using the [`AsWnfState::as_wnf_state`]
/// method.
///
/// The type parameter `T` is the type of data contained in the WNF state.
pub struct OwnedWnfState<T>
where
    T: ?Sized,
{
    pub(crate) raw: RawWnfState<T>,
}

impl<T> OwnedWnfState<T>
where
    T: ?Sized,
{
    /// Returns the name of this WNF state
    pub const fn state_name(&self) -> WnfStateName {
        self.raw.state_name()
    }

    /// Leaks this [`OwnedWnfState<T>`]
    ///
    /// This consumes the [`OwnedWnfState<T>`] without dropping it, returning a [`BorrowedWnfState<'static, T>`] with
    /// `'static` lifetime that represents the same underlying WNF state.
    ///
    /// Note that while this is named after [`Box::leak`], it doesn't leak memory, but it leaks a WNF state in the sense
    /// that the WNF state doesn't get deleted on drop.
    pub fn leak(self) -> BorrowedWnfState<'static, T> {
        BorrowedWnfState::from_raw(self.into_raw())
    }

    /// Casts the data type of this [`OwnedWnfState<T>`] to a different type `U`
    ///
    /// The returned [`OwnedWnfState<U>`] represents the same underlying WNF state, but treats it as containing data of
    /// a different type `U`.
    pub fn cast<U>(self) -> OwnedWnfState<U> {
        OwnedWnfState::from_raw(self.into_raw().cast())
    }

    /// Creates a new [`OwnedWnfState`] wrapping a given [`RawWnfState`]
    pub(crate) const fn from_raw(raw: RawWnfState<T>) -> Self {
        Self { raw }
    }

    /// Consumes this [`OwnedWnfState`] without dropping it, returning the inner [`RawWnfState`]
    pub(crate) fn into_raw(self) -> RawWnfState<T> {
        ManuallyDrop::new(self).raw
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: PartialEq<T>`
impl<T> PartialEq<Self> for OwnedWnfState<T>
where
    T: ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Eq`
impl<T> Eq for OwnedWnfState<T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Hash`
impl<T> Hash for OwnedWnfState<T>
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
impl<T> Debug for OwnedWnfState<T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnedWnfState").field("raw", &self.raw).finish()
    }
}

impl<T> Drop for OwnedWnfState<T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        let _ = self.raw.delete();
    }
}

/// A borrowed WNF state
///
/// This has a lifetime parameter to tie it to something that owns the WNF state, typically an [`OwnedWnfState<T>`].
///
/// Unlike [`OwnedWnfState`], this implements [`Copy`] (and [`Clone`]) and does not delete the represented WNF state on
/// drop.
///
/// While borrowing in Rust usually refers to borrowing memory, this applies the idea of borrowing to an external
/// entity, namely a WNF state. It's similar to [`BorrowedHandle<'a>`](std::os::windows::io::BorrowedHandle) in that
/// regard.
///
/// Calling [`Clone::clone`] on a [`BorrowedWnfState<'a, T>`] just makes a trivial copy, returning another
/// [`BorrowedWnfState<'a, T>`] with the same lifetime as the original one and representing the same underlying WNF
/// state. The same applies to the [`ToOwned::to_owned`] method.  If you want to turn a [`BorrowedWnfState<'a, T>`]
/// into an [`OwnedWnfState<T>`] (which will then delete the underlying WNF state on drop), use the
/// [`BorrowedWnfState::to_owned_wnf_state`] method.
///
/// The type parameter `T` is the type of data contained in the WNF state.
pub struct BorrowedWnfState<'a, T>
where
    T: ?Sized,
{
    pub(crate) raw: RawWnfState<T>,
    _marker: PhantomData<&'a ()>,
}

impl<'a, T> BorrowedWnfState<'a, T>
where
    T: ?Sized,
{
    /// Returns the name of this WNF state
    pub const fn state_name(self) -> WnfStateName {
        self.raw.state_name()
    }

    /// Turns this [`BorrowedWnfState<'a, T>`] into an [`OwnedWnfState<T>`] representing the same underlying WNF state
    ///
    /// Note that the underlying WNF state will be deleted when the [`OwnedWnfState<T>`] is dropped.
    pub const fn to_owned_wnf_state(self) -> OwnedWnfState<T> {
        OwnedWnfState::from_raw(self.raw)
    }

    /// Casts the data type of this [`BorrowedWnfState<'a, T>`] to a different type `U`
    ///
    /// The returned [`BorrowedWnfState<'a, U>`] represents the same underlying WNF state, but treats it as containing
    /// data of a different type `U`.
    pub const fn cast<U>(self) -> BorrowedWnfState<'a, U> {
        BorrowedWnfState::from_raw(self.raw.cast())
    }

    /// Creates a new [`BorrowedWnfState<'a, T>`] wrapping a given [`RawWnfState<T>`]
    ///
    /// The lifetime of the returned [`BorrowedWnfState<'a, T>`] is inferred at the call site.
    pub(crate) const fn from_raw(raw: RawWnfState<T>) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }
}

impl<T> BorrowedWnfState<'static, T>
where
    T: ?Sized,
{
    /// Statically borrows the WNF state with the given name
    ///
    /// Note that an underlying WNF state with the given name may or may not exist. The returned
    /// [`BorrowedWnfState<'static, T>`] having a `'static` lifetime just means that the WNF state is borrowed directly
    /// from the system rather than from an [`OwnedWnfState<T>`] that will be dropped at some point.
    pub const fn from_state_name(state_name: WnfStateName) -> Self {
        Self::from_raw(RawWnfState::from_state_name_and_type_id(state_name, TypeId::none()))
    }

    /// Statically borrows the WNF state with the given name using the given type id
    ///
    /// Note that an underlying WNF state with the given name may or may not exist. The returned
    /// [`BorrowedWnfState<'static, T>`] having a `'static` lifetime just means that the WNF state is borrowed directly
    /// from the system rather than from an [`OwnedWnfState<T>`] that will be dropped at some point.
    pub const fn from_state_name_and_type_id(state_name: WnfStateName, type_id: GUID) -> Self {
        Self::from_raw(RawWnfState::from_state_name_and_type_id(
            state_name,
            TypeId::from_guid(type_id),
        ))
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Copy`
impl<T> Copy for BorrowedWnfState<'_, T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Clone`
impl<T> Clone for BorrowedWnfState<'_, T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: PartialEq<T>`
impl<T> PartialEq<Self> for BorrowedWnfState<'_, T>
where
    T: ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Eq`
impl<T> Eq for BorrowedWnfState<'_, T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Hash`
impl<T> Hash for BorrowedWnfState<'_, T>
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
impl<T> Debug for BorrowedWnfState<'_, T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowedWnfState").field("raw", &self.raw).finish()
    }
}

/// Trait for types that can be borrowed as a WNF state
///
/// This is implemented for both [`OwnedWnfState<T>`] and [`BorrowedWnfState<'a, T>`]. There are two main use cases for
/// it:
///
/// - Functions that can accept either a reference to an owned or a borrowed WNF state: Even though a
/// [`BorrowedWnfState<'a, T>`] plays the role of a reference to a WNF state, it's not technically a reference. As a
/// consequence, there is no deref coercion for WNF states, i.e. you cannot just pass an [`&'a OwnedWnfState<T>`] where
/// a [`BorrowedWnfState<'a, T>`] is expected. In order to accept both types, functions can instead take a reference to
/// a generic type implementing [`AsWnfState`]:
/// ```
/// # use std::io;
/// # use wnf::{AsWnfState, OwnedWnfState};
/// #
/// fn add_to_state(state: &impl AsWnfState<Data = u32>, delta: u32) -> io::Result<()> {
///     let state = state.as_wnf_state();
///     let value = state.get()?;
///     let new_value = value + delta;
///     state.set(&new_value)
/// }
/// ```
///
/// - Types that can contain either an owned or a borrowed WNF state:
/// ```
/// # use std::io;
/// # use wnf::AsWnfState;
/// #
/// struct StateWrapper<S> {
///     state: S,
/// }
///
/// impl<S> StateWrapper<S>
/// where
///     S: AsWnfState<Data = u32>
/// {
///     fn add(&self, delta: u32) -> io::Result<()> {
///         let state = self.state.as_wnf_state();
///         let value = state.get()?;
///         let new_value = value + delta;
///         state.set(&new_value)
///     }
/// }
/// ```
///
/// When comparing [`OwnedWnfState<T>`] with [`OwnedHandle`](std::os::windows::io::OwnedHandle) and
/// [`BorrowedWnfState<'a, T>`] with [`BorrowedHandle<'a>`](std::os::windows::io::BorrowedHandle), this trait plays the
/// role of the [`AsHandle`](std::os::windows::io::AsHandle) trait.
///
/// This trait is sealed and cannot by implemented outside of the `wnf` crate.
pub trait AsWnfState: private::Sealed {
    /// The type of the data contained in the borrowed WNF state
    type Data: ?Sized;

    /// Borrows a value as a WNF state
    fn as_wnf_state(&self) -> BorrowedWnfState<Self::Data>;
}

impl<T> AsWnfState for OwnedWnfState<T>
where
    T: ?Sized,
{
    type Data = T;

    fn as_wnf_state(&self) -> BorrowedWnfState<T> {
        BorrowedWnfState::from_raw(self.raw)
    }
}

impl<T> AsWnfState for BorrowedWnfState<'_, T>
where
    T: ?Sized,
{
    type Data = T;

    fn as_wnf_state(&self) -> BorrowedWnfState<T> {
        *self
    }
}

/// A raw WNF state
///
/// This neither deletes the underlying WNF state on drop, nor does it have a lifetime.
pub(crate) struct RawWnfState<T>
where
    T: ?Sized,
{
    pub(crate) state_name: WnfStateName,
    pub(crate) type_id: TypeId,
    // `RawWnfState<T>` is neither covariant nor contravariant in `T` and doesn't own a `T`
    _marker: PhantomData<fn(T) -> T>,
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    /// Creates a [`RawWnfState<T>`] with the given name using the given type id
    pub(crate) const fn from_state_name_and_type_id(state_name: WnfStateName, type_id: TypeId) -> Self {
        Self {
            state_name,
            type_id,
            _marker: PhantomData,
        }
    }

    /// Returns the name of this WNF state
    const fn state_name(self) -> WnfStateName {
        self.state_name
    }

    /// Casts the data type of this [`RawWnfState<T>`] to a different type `U`
    /// The returned [`RawWnfState<U>`] represents the same underlying WNF state, but treats it as containing data of
    /// a different type `U`.
    pub(crate) const fn cast<U>(self) -> RawWnfState<U> {
        RawWnfState::from_state_name_and_type_id(self.state_name, self.type_id)
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Copy`
impl<T> Copy for RawWnfState<T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Clone`
impl<T> Clone for RawWnfState<T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: PartialEq<T>`
impl<T> PartialEq<Self> for RawWnfState<T>
where
    T: ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        // We don't compare the type id because a WNF state is already uniquely identified by its name
        self.state_name == other.state_name
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Eq`
impl<T> Eq for RawWnfState<T> where T: ?Sized {}

// We cannot derive this because that would impose an unnecessary trait bound `T: Hash`
impl<T> Hash for RawWnfState<T>
where
    T: ?Sized,
{
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.state_name.hash(state);
    }
}

// We cannot derive this because that would impose an unnecessary trait bound `T: Debug`
impl<T> Debug for RawWnfState<T>
where
    T: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawWnfState")
            .field("state_name", &self.state_name)
            .field("type_id", &self.type_id)
            .finish()
    }
}

mod private {
    use super::{BorrowedWnfState, OwnedWnfState};

    pub trait Sealed {}

    impl<T> Sealed for OwnedWnfState<T> where T: ?Sized {}
    impl<T> Sealed for BorrowedWnfState<'_, T> where T: ?Sized {}
}
