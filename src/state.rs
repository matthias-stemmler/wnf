use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;

use crate::state_name::WnfStateName;

pub struct OwnedWnfState<T> {
    pub(crate) raw: RawWnfState<T>,
}

impl<T> PartialEq<Self> for OwnedWnfState<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T> Eq for OwnedWnfState<T> {}

impl<T> Hash for OwnedWnfState<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.raw.hash(state);
    }
}

impl<T> Debug for OwnedWnfState<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnedWnfState")
            .field("state_name", &self.state_name())
            .finish()
    }
}

impl<T> OwnedWnfState<T> {
    pub fn state_name(&self) -> WnfStateName {
        self.raw.state_name()
    }

    pub fn borrow(&self) -> BorrowedWnfState<T> {
        BorrowedWnfState::from_raw(self.raw)
    }

    pub fn leak(self) -> BorrowedWnfState<'static, T> {
        BorrowedWnfState::from_raw(self.into_raw())
    }

    pub fn cast<U>(self) -> OwnedWnfState<U> {
        OwnedWnfState::from_raw(self.into_raw().cast())
    }

    pub(crate) fn from_raw(raw: RawWnfState<T>) -> Self {
        Self { raw }
    }

    pub(crate) fn into_raw(self) -> RawWnfState<T> {
        ManuallyDrop::new(self).raw
    }
}

impl<T> Drop for OwnedWnfState<T> {
    fn drop(&mut self) {
        let _ = self.raw.delete();
    }
}

pub struct BorrowedWnfState<'a, T> {
    pub(crate) raw: RawWnfState<T>,
    _marker: PhantomData<&'a ()>,
}

impl<T> Copy for BorrowedWnfState<'_, T> {}

impl<T> Clone for BorrowedWnfState<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq<Self> for BorrowedWnfState<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T> Eq for BorrowedWnfState<'_, T> {}

impl<T> Hash for BorrowedWnfState<'_, T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.raw.hash(state);
    }
}

impl<T> Debug for BorrowedWnfState<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BorrowedWnfState")
            .field("state_name", &self.state_name())
            .finish()
    }
}

impl<'a, T> BorrowedWnfState<'a, T> {
    pub fn state_name(&self) -> WnfStateName {
        self.raw.state_name()
    }

    pub fn into_owned(self) -> OwnedWnfState<T> {
        OwnedWnfState::from_raw(self.into_raw())
    }

    pub fn cast<U>(self) -> BorrowedWnfState<'a, U> {
        BorrowedWnfState::from_raw(self.into_raw().cast())
    }

    pub(crate) fn from_raw(raw: RawWnfState<T>) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    pub(crate) fn into_raw(self) -> RawWnfState<T> {
        self.raw
    }
}

impl<T> BorrowedWnfState<'static, T> {
    pub fn from_state_name(state_name: WnfStateName) -> Self {
        Self::from_raw(RawWnfState::from_state_name(state_name))
    }
}

pub(crate) struct RawWnfState<T> {
    pub(crate) state_name: WnfStateName,
    _marker: PhantomData<fn(T) -> T>,
}

impl<T> Copy for RawWnfState<T> {}

impl<T> Clone for RawWnfState<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq<Self> for RawWnfState<T> {
    fn eq(&self, other: &Self) -> bool {
        self.state_name == other.state_name
    }
}

impl<T> Eq for RawWnfState<T> {}

impl<T> Hash for RawWnfState<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.state_name.hash(state);
    }
}

impl<T> Debug for RawWnfState<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawWnfState")
            .field("state_name", &self.state_name)
            .finish()
    }
}

impl<T> RawWnfState<T> {
    pub(crate) fn from_state_name(state_name: WnfStateName) -> Self {
        Self {
            state_name,
            _marker: PhantomData,
        }
    }

    pub(crate) fn state_name(self) -> WnfStateName {
        self.state_name
    }

    pub(crate) fn cast<U>(self) -> RawWnfState<U> {
        RawWnfState::from_state_name(self.state_name)
    }
}
