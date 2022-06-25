use std::borrow::Borrow;

use crate::read::WnfReadBoxed;
use crate::state::RawWnfState;
use crate::{BorrowedWnfState, NoUninit, OwnedWnfState, WnfApplyError, WnfRead};

impl<T> OwnedWnfState<T>
where
    T: WnfRead + NoUninit,
{
    pub fn replace<D>(&self, new_value: D) -> Result<T, WnfApplyError>
    where
        D: Borrow<T>,
    {
        self.raw.replace(new_value)
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfReadBoxed + NoUninit + ?Sized,
{
    pub fn replace_boxed<D>(&self, new_value: D) -> Result<Box<T>, WnfApplyError>
    where
        D: Borrow<T>,
    {
        self.raw.replace_boxed(new_value)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead + NoUninit,
{
    pub fn replace<D>(&self, new_value: D) -> Result<T, WnfApplyError>
    where
        D: Borrow<T>,
    {
        self.raw.replace(new_value)
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfReadBoxed + NoUninit + ?Sized,
{
    pub fn replace_boxed<D>(&self, new_value: D) -> Result<Box<T>, WnfApplyError>
    where
        D: Borrow<T>,
    {
        self.raw.replace_boxed(new_value)
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead + NoUninit,
{
    pub fn replace<D>(&self, new_value: D) -> Result<T, WnfApplyError>
    where
        D: Borrow<T>,
    {
        let mut old_value = None;
        self.apply(|value, _| {
            old_value = Some(value);
            new_value.borrow()
        })?;
        Ok(old_value.unwrap())
    }
}

impl<T> RawWnfState<T>
where
    T: WnfReadBoxed + NoUninit + ?Sized,
{
    pub fn replace_boxed<D>(&self, new_value: D) -> Result<Box<T>, WnfApplyError>
    where
        D: Borrow<T>,
    {
        let mut old_value = None;
        self.apply_boxed(|value, _| {
            old_value = Some(value);
            new_value.borrow()
        })?;
        Ok(old_value.unwrap())
    }
}
