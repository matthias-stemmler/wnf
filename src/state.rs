use std::borrow::Borrow;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;

use crate::bytes::{CheckedBitPattern, NoUninit};
use crate::callback::WnfCallback;
use crate::error::{WnfApplyError, WnfDeleteError, WnfInfoError, WnfQueryError, WnfSubscribeError, WnfUpdateError};
use crate::raw_state::RawWnfState;
use crate::subscription::WnfSubscriptionHandle;
use crate::{WnfChangeStamp, WnfCreateError, WnfStampedData, WnfStateName};

pub struct OwnedWnfState<T> {
    raw: RawWnfState<T>,
}

pub struct BorrowedWnfState<'a, T> {
    raw: RawWnfState<T>,
    _marker: PhantomData<&'a ()>,
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

    pub fn exists(&self) -> Result<bool, WnfInfoError> {
        self.raw.exists()
    }

    pub fn subscribers_present(&self) -> Result<bool, WnfInfoError> {
        self.raw.subscribers_present()
    }

    pub fn is_quiescent(&self) -> Result<bool, WnfInfoError> {
        self.raw.is_quiescent()
    }

    pub fn create_temporary() -> Result<Self, WnfCreateError> {
        RawWnfState::create_temporary().map(Self::from_raw)
    }

    pub fn delete(self) -> Result<(), WnfDeleteError> {
        self.into_raw().delete()
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

    fn from_raw(raw: RawWnfState<T>) -> Self {
        Self { raw }
    }

    fn into_raw(self) -> RawWnfState<T> {
        ManuallyDrop::new(self).raw
    }
}

impl<T> OwnedWnfState<T>
where
    T: CheckedBitPattern,
{
    pub fn get(&self) -> Result<T, WnfQueryError> {
        self.raw.get()
    }

    pub fn get_boxed(&self) -> Result<Box<T>, WnfQueryError> {
        self.raw.get_boxed()
    }

    pub fn get_slice(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.raw.get_slice()
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        self.raw.query()
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError> {
        self.raw.query_boxed()
    }

    pub fn query_slice(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError> {
        self.raw.query_slice()
    }
}

impl<T> OwnedWnfState<T>
where
    T: NoUninit,
{
    pub fn set<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.raw.set(data)
    }

    pub fn set_slice<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<[T]>,
    {
        self.raw.set_slice(data)
    }

    pub fn update<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.raw.update(data, expected_change_stamp)
    }

    pub fn update_slice<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<[T]>,
    {
        self.raw.update_slice(data, expected_change_stamp)
    }
}

impl<T> OwnedWnfState<T>
where
    T: CheckedBitPattern + NoUninit,
{
    pub fn apply<D, F>(&self, transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(T) -> Option<D>,
    {
        self.raw.apply(transform)
    }

    pub fn apply_boxed<D, F>(&self, transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Option<D>,
    {
        self.raw.apply_boxed(transform)
    }

    pub fn apply_slice<D, F>(&self, transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Option<D>,
    {
        self.raw.apply_slice(transform)
    }

    pub fn try_apply<D, E, F>(&self, tranform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(T) -> Result<Option<D>, E>,
    {
        self.raw.try_apply(tranform)
    }

    pub fn try_apply_boxed<D, E, F>(&self, transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Result<Option<D>, E>,
    {
        self.raw.try_apply_boxed(transform)
    }

    pub fn try_apply_slice<D, E, F>(&self, transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Result<Option<D>, E>,
    {
        self.raw.try_apply_slice(transform)
    }
}

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

impl<T> Drop for OwnedWnfState<T> {
    fn drop(&mut self) {
        let _ = self.raw.delete();
    }
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

    pub fn exists(&self) -> Result<bool, WnfInfoError> {
        self.raw.exists()
    }

    pub fn subscribers_present(&self) -> Result<bool, WnfInfoError> {
        self.raw.subscribers_present()
    }

    pub fn is_quiescent(&self) -> Result<bool, WnfInfoError> {
        self.raw.is_quiescent()
    }

    pub fn into_owned(self) -> OwnedWnfState<T> {
        OwnedWnfState::from_raw(self.into_raw())
    }

    pub fn delete(self) -> Result<(), WnfDeleteError> {
        self.into_raw().delete()
    }

    pub fn cast<U>(self) -> BorrowedWnfState<'a, U> {
        BorrowedWnfState::from_raw(self.into_raw().cast())
    }

    fn from_raw(raw: RawWnfState<T>) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    fn into_raw(self) -> RawWnfState<T> {
        self.raw
    }
}

impl<T> BorrowedWnfState<'static, T> {
    pub fn from_state_name(state_name: WnfStateName) -> Self {
        Self::from_raw(RawWnfState::from_state_name(state_name))
    }
}

impl<'a, T> BorrowedWnfState<'a, T>
where
    T: CheckedBitPattern,
{
    pub fn get(&self) -> Result<T, WnfQueryError> {
        self.raw.get()
    }

    pub fn get_boxed(&self) -> Result<Box<T>, WnfQueryError> {
        self.raw.get_boxed()
    }

    pub fn get_slice(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.raw.get_slice()
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        self.raw.query()
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError> {
        self.raw.query_boxed()
    }

    pub fn query_slice(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError> {
        self.raw.query_slice()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: NoUninit,
{
    pub fn set<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.raw.set(data)
    }

    pub fn set_slice<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<[T]>,
    {
        self.raw.set_slice(data)
    }

    pub fn update<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.raw.update(data, expected_change_stamp)
    }

    pub fn update_slice<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<[T]>,
    {
        self.raw.update_slice(data, expected_change_stamp)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: CheckedBitPattern + NoUninit,
{
    pub fn apply<D, F>(&self, transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(T) -> Option<D>,
    {
        self.raw.apply(transform)
    }

    pub fn apply_boxed<D, F>(&self, transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Option<D>,
    {
        self.raw.apply_boxed(transform)
    }

    pub fn apply_slice<D, F>(&self, transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Option<D>,
    {
        self.raw.apply_slice(transform)
    }

    pub fn try_apply<D, E, F>(&self, transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(T) -> Result<Option<D>, E>,
    {
        self.raw.try_apply(transform)
    }

    pub fn try_apply_boxed<D, E, F>(&self, transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Result<Option<D>, E>,
    {
        self.raw.try_apply_boxed(transform)
    }

    pub fn try_apply_slice<D, E, F>(&self, transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Result<Option<D>, E>,
    {
        self.raw.try_apply_slice(transform)
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
