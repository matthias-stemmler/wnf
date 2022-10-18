//! Types dealing with the data of a WNF state

#![deny(unsafe_code)]

use std::fmt;
use std::fmt::{Display, Formatter};

/// Placeholder for data of a WNF state that you don't care about
///
/// This type is zero-sized but can be "read" from any WNF state regardless of the size of the state's actual data. This
/// is useful, for instance, if you want to check if a state can be read from without knowing what its actual data looks
/// like:
/// ```
/// # use std::io;
/// # use wnf::{AsWnfState, OwnedWnfState, WnfChangeStamp, WnfOpaqueData, WnfStampedData};
/// #
/// fn can_read(state: impl AsWnfState) -> bool {
///     // If we replaced `WnfOpaqueData` by, say, `()`, this would only work if the data was actually zero-sized
///     // With `WnfOpaqueData`, it works in any case
///     state.as_wnf_state().cast::<WnfOpaqueData>().get().is_ok()
/// }
///
/// // Here we could have used any type `T: NoUninit` in place of `u32`
/// let state = OwnedWnfState::<u32>::create_temporary().expect("Failed to create WNF state");
/// state.set(&42).expect("Failed to set WNF state data");
///
/// assert!(can_read(state));
/// ```
///
/// Another use case is reading the change stamp of a state without knowing what the actual data looks like, but that is
/// already implemented in [`OwnedWnfState::change_stamp`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WnfOpaqueData {
    _private: (),
}

impl WnfOpaqueData {
    /// Creates a new [`WnfOpaqueData`]
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

/// The change stamp of a WNF state
///
/// This is `0` when the state is created and is increased by `1` on every update to the state.
///
/// Several methods on other types deal with change stamps. See their documentations for details:
/// - [`OwnedWnfState::change_stamp`]
/// - [`OwnedWnfState::query`] and [`OwnedWnfState::query_boxed`]
/// - [`OwnedWnfState::update`]
/// - [`WnfDataAccessor::change_stamp`]
/// - [`WnfDataAccessor::query`] and [`WnfDataAccessor::query_boxed`]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WnfChangeStamp(u32);

impl WnfChangeStamp {
    /// Returns the initial change stamp of every WNF state, which is `0`:
    ///
    /// ```
    /// # use wnf::WnfChangeStamp;
    /// #
    /// assert_eq!(WnfChangeStamp::initial().value(), 0);
    /// ```
    pub const fn initial() -> Self {
        Self(0)
    }

    /// Returns the inner value
    pub const fn value(&self) -> u32 {
        self.0
    }

    /// Returns a mutable raw pointer to the inner value for use in FFI
    pub(crate) fn as_mut_ptr(&mut self) -> *mut u32 {
        &mut self.0
    }
}

impl From<u32> for WnfChangeStamp {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<WnfChangeStamp> for u32 {
    fn from(WnfChangeStamp(value): WnfChangeStamp) -> Self {
        value
    }
}

impl Display for WnfChangeStamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// WNF state data of type `T` together with a change stamp
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WnfStampedData<T> {
    data: T,
    change_stamp: WnfChangeStamp,
}

impl<T> WnfStampedData<T> {
    /// Creates a new [`WnfStampedData`] from the given data and change stamp
    pub fn from_data_change_stamp(data: T, change_stamp: impl Into<WnfChangeStamp>) -> Self {
        Self {
            data,
            change_stamp: change_stamp.into(),
        }
    }

    /// Moves the contained data and change stamp out of this [`WnfStampedData`]
    pub fn into_data_change_stamp(self) -> (T, WnfChangeStamp) {
        (self.data, self.change_stamp)
    }

    /// Moves the contained data out of this [`WnfStampedData`], discarding the change stamp
    pub fn into_data(self) -> T {
        self.data
    }

    /// Returns a reference to the data contained in this [`WnfStampedData`]
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Returns the change stamp contained in this [`WnfStampedData`]
    pub fn change_stamp(&self) -> WnfChangeStamp {
        self.change_stamp
    }

    /// Returns a new [`WnfStampedData`] obtained by applying a given closure to the contained data
    pub fn map<U, F>(self, op: F) -> WnfStampedData<U>
    where
        F: FnOnce(T) -> U,
    {
        WnfStampedData {
            data: op(self.data),
            change_stamp: self.change_stamp,
        }
    }
}

impl<T> From<(T, WnfChangeStamp)> for WnfStampedData<T> {
    fn from((data, change_stamp): (T, WnfChangeStamp)) -> Self {
        Self { data, change_stamp }
    }
}

impl<T> From<WnfStampedData<T>> for (T, WnfChangeStamp) {
    fn from(stamped_data: WnfStampedData<T>) -> Self {
        (stamped_data.data, stamped_data.change_stamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_stamp_display() {
        assert_eq!(format!("{}", WnfChangeStamp::from(42)), "42");
    }

    #[test]
    fn stamped_data_map() {
        assert_eq!(
            WnfStampedData::from_data_change_stamp(42, 1).map(|x| x.to_string()),
            WnfStampedData::from_data_change_stamp(String::from("42"), 1)
        );
    }
}
