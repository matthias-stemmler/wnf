use std::borrow::Borrow;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;

use crate::bytes::{CheckedBitPattern, NoUninit};
use crate::data::WnfStateInfo;
use crate::error::{WnfApplyError, WnfDeleteError, WnfInfoError, WnfQueryError, WnfSubscribeError, WnfUpdateError};
use crate::raw_state::RawWnfState;
use crate::subscription::WnfSubscriptionHandle;
use crate::{WnfChangeStamp, WnfCreateError, WnfStampedData, WnfStateName};

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct OwnedWnfState {
    raw: RawWnfState,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BorrowedWnfState<'a> {
    raw: RawWnfState,
    _marker: PhantomData<&'a ()>,
}

impl OwnedWnfState {
    pub fn state_name(&self) -> WnfStateName {
        self.raw.state_name()
    }

    pub fn exists(&self) -> Result<bool, WnfInfoError> {
        self.raw.exists()
    }

    pub fn info(&self) -> Result<Option<WnfStateInfo>, WnfInfoError> {
        self.raw.info()
    }

    pub fn create_temporary() -> Result<Self, WnfCreateError> {
        RawWnfState::create_temporary().map(Self::from_raw)
    }

    pub fn delete(self) -> Result<(), WnfDeleteError> {
        self.into_raw().delete()
    }

    pub fn borrow(&self) -> BorrowedWnfState {
        BorrowedWnfState::from_raw(self.raw)
    }

    pub fn leak(self) -> BorrowedWnfState<'static> {
        BorrowedWnfState::from_raw(self.into_raw())
    }

    fn from_raw(raw: RawWnfState) -> Self {
        Self { raw }
    }

    fn into_raw(self) -> RawWnfState {
        ManuallyDrop::new(self).raw
    }

    pub fn get<T>(&self) -> Result<T, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.raw.get()
    }

    pub fn get_boxed<T>(&self) -> Result<Box<T>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.raw.get_boxed()
    }

    pub fn get_slice<T>(&self) -> Result<Box<[T]>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.raw.get_slice()
    }

    pub fn query<T>(&self) -> Result<WnfStampedData<T>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.raw.query()
    }

    pub fn query_boxed<T>(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.raw.query_boxed()
    }

    pub fn query_slice<T>(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.raw.query_slice()
    }

    pub fn set<T, D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        T: NoUninit,
        D: Borrow<T>,
    {
        self.raw.set(data)
    }

    pub fn set_slice<T, D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        T: NoUninit,
        D: Borrow<[T]>,
    {
        self.raw.set_slice(data)
    }

    pub fn update<T, D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        T: NoUninit,
        D: Borrow<T>,
    {
        self.raw.update(data, expected_change_stamp)
    }

    pub fn update_slice<T, D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        T: NoUninit,
        D: Borrow<[T]>,
    {
        self.raw.update_slice(data, expected_change_stamp)
    }

    pub fn apply<T, D, F>(&self, transform: F) -> Result<(), WnfApplyError>
    where
        T: CheckedBitPattern + NoUninit,
        D: Borrow<T>,
        F: FnMut(T) -> Option<D>,
    {
        self.raw.apply(transform)
    }

    pub fn apply_boxed<T, D, F>(&self, transform: F) -> Result<(), WnfApplyError>
    where
        T: CheckedBitPattern + NoUninit,
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Option<D>,
    {
        self.raw.apply_boxed(transform)
    }

    pub fn apply_slice<T, D, F>(&self, transform: F) -> Result<(), WnfApplyError>
    where
        T: CheckedBitPattern + NoUninit,
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Option<D>,
    {
        self.raw.apply_slice(transform)
    }

    pub fn try_apply<T, D, E, F>(&self, tranform: F) -> Result<(), WnfApplyError<E>>
    where
        T: CheckedBitPattern + NoUninit,
        D: Borrow<T>,
        F: FnMut(T) -> Result<Option<D>, E>,
    {
        self.raw.try_apply(tranform)
    }

    pub fn try_apply_boxed<T, D, E, F>(&self, transform: F) -> Result<(), WnfApplyError<E>>
    where
        T: CheckedBitPattern + NoUninit,
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Result<Option<D>, E>,
    {
        self.raw.try_apply_boxed(transform)
    }

    pub fn try_apply_slice<T, D, E, F>(&self, transform: F) -> Result<(), WnfApplyError<E>>
    where
        T: CheckedBitPattern + NoUninit,
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Result<Option<D>, E>,
    {
        self.raw.try_apply_slice(transform)
    }

    pub fn subscribe<T, F>(&self, listener: Box<F>) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        T: CheckedBitPattern,
        F: FnMut(Option<WnfStampedData<T>>) + Send + ?Sized + 'static,
    {
        self.raw.subscribe(listener)
    }

    pub fn subscribe_slice_boxed<T, F>(&self, listener: Box<F>) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        T: CheckedBitPattern,
        F: FnMut(Option<WnfStampedData<Box<T>>>) + Send + ?Sized + 'static,
    {
        self.raw.subscribe_boxed(listener)
    }

    pub fn subscribe_slice<T, F>(&self, listener: Box<F>) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        T: CheckedBitPattern,
        F: FnMut(Option<WnfStampedData<Box<[T]>>>) + Send + ?Sized + 'static,
    {
        self.raw.subscribe_slice(listener)
    }
}

impl Drop for OwnedWnfState {
    fn drop(&mut self) {
        let _ = self.raw.delete();
    }
}

impl BorrowedWnfState<'_> {
    pub fn state_name(&self) -> WnfStateName {
        self.raw.state_name()
    }

    pub fn exists(&self) -> Result<bool, WnfInfoError> {
        self.raw.exists()
    }

    pub fn info(&self) -> Result<Option<WnfStateInfo>, WnfInfoError> {
        self.raw.info()
    }

    pub fn into_owned(self) -> OwnedWnfState {
        OwnedWnfState::from_raw(self.into_raw())
    }

    pub fn delete(self) -> Result<(), WnfDeleteError> {
        self.into_raw().delete()
    }

    fn from_raw(raw: RawWnfState) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    fn into_raw(self) -> RawWnfState {
        self.raw
    }
}

impl BorrowedWnfState<'static> {
    pub fn from_state_name(state_name: WnfStateName) -> Self {
        Self::from_raw(RawWnfState::from_state_name(state_name))
    }
}

impl<'a> BorrowedWnfState<'a> {
    pub fn get<T>(&self) -> Result<T, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.raw.get()
    }

    pub fn get_boxed<T>(&self) -> Result<Box<T>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.raw.get_boxed()
    }

    pub fn get_slice<T>(&self) -> Result<Box<[T]>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.raw.get_slice()
    }

    pub fn query<T>(&self) -> Result<WnfStampedData<T>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.raw.query()
    }

    pub fn query_boxed<T>(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.raw.query_boxed()
    }

    pub fn query_slice<T>(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.raw.query_slice()
    }

    pub fn set<T, D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        T: NoUninit,
        D: Borrow<T>,
    {
        self.raw.set(data)
    }

    pub fn set_slice<T, D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        T: NoUninit,
        D: Borrow<[T]>,
    {
        self.raw.set_slice(data)
    }

    pub fn update<T, D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        T: NoUninit,
        D: Borrow<T>,
    {
        self.raw.update(data, expected_change_stamp)
    }

    pub fn update_slice<T, D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        T: NoUninit,
        D: Borrow<[T]>,
    {
        self.raw.update_slice(data, expected_change_stamp)
    }

    pub fn apply<T, D, F>(&self, transform: F) -> Result<(), WnfApplyError>
    where
        T: CheckedBitPattern + NoUninit,
        D: Borrow<T>,
        F: FnMut(T) -> Option<D>,
    {
        self.raw.apply(transform)
    }

    pub fn apply_boxed<T, D, F>(&self, transform: F) -> Result<(), WnfApplyError>
    where
        T: CheckedBitPattern + NoUninit,
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Option<D>,
    {
        self.raw.apply_boxed(transform)
    }

    pub fn apply_slice<T, D, F>(&self, transform: F) -> Result<(), WnfApplyError>
    where
        T: CheckedBitPattern + NoUninit,
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Option<D>,
    {
        self.raw.apply_slice(transform)
    }

    pub fn try_apply<T, D, E, F>(&self, transform: F) -> Result<(), WnfApplyError<E>>
    where
        T: CheckedBitPattern + NoUninit,
        D: Borrow<T>,
        F: FnMut(T) -> Result<Option<D>, E>,
    {
        self.raw.try_apply(transform)
    }

    pub fn try_apply_boxed<T, D, E, F>(&self, transform: F) -> Result<(), WnfApplyError<E>>
    where
        T: CheckedBitPattern + NoUninit,
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Result<Option<D>, E>,
    {
        self.raw.try_apply_boxed(transform)
    }

    pub fn try_apply_slice<T, D, E, F>(&self, transform: F) -> Result<(), WnfApplyError<E>>
    where
        T: CheckedBitPattern + NoUninit,
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Result<Option<D>, E>,
    {
        self.raw.try_apply_slice(transform)
    }

    pub fn subscribe<T, F>(&self, listener: Box<F>) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        T: CheckedBitPattern,
        F: FnMut(Option<WnfStampedData<T>>) + Send + ?Sized + 'static,
    {
        self.raw.subscribe(listener)
    }

    pub fn subscribe_boxed<T, F>(&self, listener: Box<F>) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        T: CheckedBitPattern,
        F: FnMut(Option<WnfStampedData<Box<T>>>) + Send + ?Sized + 'static,
    {
        self.raw.subscribe_boxed(listener)
    }

    pub fn subscribe_slice<T, F>(&self, listener: Box<F>) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        T: CheckedBitPattern,
        F: FnMut(Option<WnfStampedData<Box<[T]>>>) + Send + ?Sized + 'static,
    {
        self.raw.subscribe_slice(listener)
    }
}
