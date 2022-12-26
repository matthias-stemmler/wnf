//! Methods for updating state data
//!
//! This module only adds inherent impls to [`OwnedState<T>`] and [`BorrowedState<'_, T>`](BorrowedState).

use std::ffi::c_void;
use std::{io, mem, ptr};

use tracing::debug;
use windows::Win32::Foundation::{NTSTATUS, STATUS_UNSUCCESSFUL};

use crate::bytes::NoUninit;
use crate::data::ChangeStamp;
use crate::ntapi;
use crate::state::{BorrowedState, OwnedState, RawState};

impl<T> OwnedState<T>
where
    T: NoUninit + ?Sized,
{
    /// Updates the data of this state with the given value
    ///
    /// The update is performed regardless of the current change stamp of the state. In order to perform the update
    /// conditionally based on the change stamp, use the [`update`](OwnedState::update) method.
    ///
    /// # Errors
    /// Returns an error if updating fails
    pub fn set(&self, data: &T) -> io::Result<()> {
        self.raw.set(data)
    }

    /// Updates the data of this state with the given value
    ///
    /// The update is only performed if the change stamp of the state before the update matches the given
    /// `expected_change_stamp`. In this case, the method returns `true`. Otherwise, the update is not performed and the
    /// method returns `false`.
    ///
    /// Note that this check is not guaranteed to work reliably in all situations. If the size of the given data exceeds
    /// the internal capacity of the state (causing a reallocation) while there is another concurrent update, it may
    /// happen that the data is updated even though the change stamp is already greater than the given one.
    ///
    /// In order to update the data without checking the change stamp, use the [`set`](OwnedState::set) method.
    ///
    /// # Errors
    /// Returns an error if updating fails
    pub fn update(&self, data: &T, expected_change_stamp: impl Into<ChangeStamp>) -> io::Result<bool> {
        self.raw.update(data, expected_change_stamp.into())
    }
}

impl<T> BorrowedState<'_, T>
where
    T: NoUninit + ?Sized,
{
    /// Updates the data of this state with the given value
    ///
    /// See [`OwnedState::set`]
    pub fn set(self, data: &T) -> io::Result<()> {
        self.raw.set(data)
    }

    /// Updates the data of this state with the given value
    ///
    /// See [`OwnedState::update`]
    pub fn update(self, data: &T, expected_change_stamp: impl Into<ChangeStamp>) -> io::Result<bool> {
        self.raw.update(data, expected_change_stamp.into())
    }
}

impl<T> RawState<T>
where
    T: NoUninit + ?Sized,
{
    /// Updates the data of this state with the given value
    ///
    /// The update is performed regardless of the current change stamp of the state.
    fn set(self, data: &T) -> io::Result<()> {
        self.update_internal(data, None).ok()?;
        Ok(())
    }

    /// Updates the data of this state with the given value
    ///
    /// The update is only performed if the change stamp of the state before the update matches the given
    /// `expected_change_stamp`. In this case, the method returns `true`. Otherwise, the update is not performed and the
    /// method returns `false`.
    pub(crate) fn update(self, data: &T, expected_change_stamp: ChangeStamp) -> io::Result<bool> {
        let result = self.update_internal(data, Some(expected_change_stamp));

        Ok(if result == STATUS_UNSUCCESSFUL {
            false
        } else {
            result.ok()?;
            true
        })
    }

    fn update_internal(self, data: &T, expected_change_stamp: Option<ChangeStamp>) -> NTSTATUS {
        let buffer_size = mem::size_of_val(data) as u32;
        let matching_change_stamp = expected_change_stamp.unwrap_or_default().into();
        let check_stamp: u32 = expected_change_stamp.is_some().into();

        // SAFETY:
        // - The pointer in the first argument points to a valid `u64` because it comes from a live reference
        // - The pointer in the second argument is valid for reads of size `buffer_size` because it comes from a live
        //   reference `data` (of type `T`) and `buffer_size == mem::size_of_val(data)`
        // - The memory range of size `buffer_size` starting at `buffer` is initialized because `T: NoUninit`
        // - The pointer in the fourth argument is either a null pointer or points to a valid `GUID` by the guarantees
        //   of `TypeId::as_ptr`
        let result = unsafe {
            ntapi::NtUpdateWnfStateData(
                &self.state_name.opaque_value(),
                data as *const T as *const c_void,
                buffer_size,
                self.type_id.as_ptr(),
                ptr::null(),
                matching_change_stamp,
                check_stamp,
            )
        };

        debug!(
            target: ntapi::TRACING_TARGET,
            ?result,
            input.state_name = %self.state_name,
            input.buffer_size = buffer_size,
            input.type_id = %self.type_id,
            input.matching_change_stamp = matching_change_stamp,
            input.check_stamp = check_stamp,
            "NtUpdateWnfStateData",
        );

        result
    }
}
