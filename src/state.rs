use std::marker::PhantomData;
use std::mem::ManuallyDrop;

use crate::data::WnfStateInfo;
use crate::error::{WnfApplyError, WnfDeleteError, WnfInfoError, WnfQueryError, WnfSubscribeError, WnfUpdateError};
use crate::raw_state::RawWnfState;
use crate::subscription::WnfSubscriptionHandle;
use crate::{Pod, WnfChangeStamp, WnfCreateError, WnfStampedData, WnfStateName};

// conceptually: Box<State<T>>
#[derive(Debug)]
pub struct OwnedWnfState<T> {
    raw: RawWnfState<T>,
}

// conceptually: &'a State<T>
#[derive(Clone, Copy, Debug)]
pub struct BorrowedWnfState<'a, T> {
    raw: RawWnfState<T>,
    _marker: PhantomData<&'a ()>,
}

impl<T> OwnedWnfState<T> {
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

    pub fn borrow(&self) -> BorrowedWnfState<'_, T> {
        BorrowedWnfState::from_raw(self.raw)
    }

    pub fn leak(self) -> BorrowedWnfState<'static, T> {
        BorrowedWnfState::from_raw(self.into_raw())
    }

    fn from_raw(raw: RawWnfState<T>) -> Self {
        Self { raw }
    }

    fn into_raw(self) -> RawWnfState<T> {
        ManuallyDrop::new(self).raw
    }
}

impl<T: Pod> OwnedWnfState<T> {
    pub fn get(&self) -> Result<T, WnfQueryError> {
        self.raw.get()
    }

    pub fn get_slice(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.raw.get_slice()
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        self.raw.query()
    }

    pub fn query_slice(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError> {
        self.raw.query_slice()
    }

    pub fn set(&self, data: &T) -> Result<(), WnfUpdateError> {
        self.raw.set(data)
    }

    pub fn set_slice(&self, data: &[T]) -> Result<(), WnfUpdateError> {
        self.raw.set_slice(data)
    }

    pub fn update(&self, data: &T, expected_change_stamp: Option<WnfChangeStamp>) -> Result<bool, WnfUpdateError> {
        self.raw.update(data, expected_change_stamp)
    }

    pub fn update_slice(
        &self,
        data: &[T],
        expected_change_stamp: Option<WnfChangeStamp>,
    ) -> Result<bool, WnfUpdateError> {
        self.raw.update_slice(data, expected_change_stamp)
    }

    pub fn apply(&self, op: impl FnMut(&T) -> T) -> Result<(), WnfApplyError> {
        self.raw.apply(op)
    }

    pub fn apply_slice(&self, op: impl FnMut(&[T]) -> Box<[T]>) -> Result<(), WnfApplyError> {
        self.raw.apply_slice(op)
    }

    pub fn subscribe<F: FnMut(Option<WnfStampedData<&T>>) + Send + ?Sized + 'static>(
        &self,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<'_, F>, WnfSubscribeError> {
        self.raw.subscribe(listener)
    }

    pub fn subscribe_slice<F: FnMut(Option<WnfStampedData<&[T]>>) + Send + ?Sized + 'static>(
        &self,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<'_, F>, WnfSubscribeError> {
        self.raw.subscribe_slice(listener)
    }
}

impl<T> Drop for OwnedWnfState<T> {
    fn drop(&mut self) {
        let _ = self.raw.delete();
    }
}

impl<T> BorrowedWnfState<'_, T> {
    pub fn state_name(&self) -> WnfStateName {
        self.raw.state_name()
    }

    pub fn exists(&self) -> Result<bool, WnfInfoError> {
        self.raw.exists()
    }

    pub fn info(&self) -> Result<Option<WnfStateInfo>, WnfInfoError> {
        self.raw.info()
    }

    pub fn into_owned(self) -> OwnedWnfState<T> {
        OwnedWnfState::from_raw(self.into_raw())
    }

    pub fn delete(self) -> Result<(), WnfDeleteError> {
        self.into_raw().delete()
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

impl<'a, T: Pod> BorrowedWnfState<'a, T> {
    pub fn get(&self) -> Result<T, WnfQueryError> {
        self.raw.get()
    }

    pub fn get_slice(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.raw.get_slice()
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        self.raw.query()
    }

    pub fn query_slice(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError> {
        self.raw.query_slice()
    }

    pub fn set(&self, data: &T) -> Result<(), WnfUpdateError> {
        self.raw.set(data)
    }

    pub fn set_slice(&self, data: &[T]) -> Result<(), WnfUpdateError> {
        self.raw.set_slice(data)
    }

    pub fn update(&self, data: &T, expected_change_stamp: Option<WnfChangeStamp>) -> Result<bool, WnfUpdateError> {
        self.raw.update(data, expected_change_stamp)
    }

    pub fn update_slice(
        &self,
        data: &[T],
        expected_change_stamp: Option<WnfChangeStamp>,
    ) -> Result<bool, WnfUpdateError> {
        self.raw.update_slice(data, expected_change_stamp)
    }

    pub fn apply(&self, op: impl FnMut(&T) -> T) -> Result<(), WnfApplyError> {
        self.raw.apply(op)
    }

    pub fn apply_slice(&self, op: impl FnMut(&[T]) -> Box<[T]>) -> Result<(), WnfApplyError> {
        self.raw.apply_slice(op)
    }

    pub fn subscribe<F: FnMut(Option<WnfStampedData<&T>>) + Send + ?Sized + 'static>(
        &self,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<'a, F>, WnfSubscribeError> {
        self.raw.subscribe(listener)
    }

    pub fn subscribe_slice<F: FnMut(Option<WnfStampedData<&[T]>>) + Send + ?Sized + 'static>(
        &self,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<'a, F>, WnfSubscribeError> {
        self.raw.subscribe_slice(listener)
    }
}
