//! Types dealing with the data of a state

#![deny(unsafe_code)]

use std::fmt;
use std::fmt::{Display, Formatter};

/// Placeholder for data of a state that you don't care about
///
/// This type is zero-sized but can be "read" from any state regardless of the size of the state's actual data. This
/// is useful, for instance, if you want to check if a state can be read from without knowing what its actual data looks
/// like:
/// ```
/// # use std::io;
/// # use wnf::{AsState, OwnedState, ChangeStamp, OpaqueData, StampedData};
/// #
/// fn can_read(state: impl AsState) -> bool {
///     // If we replaced `OpaqueData` by, say, `()`, this would only work if the data was actually zero-sized
///     // With `OpaqueData`, it works in any case
///     state.as_state().cast::<OpaqueData>().get().is_ok()
/// }
///
/// // Here we could have used any type `T: NoUninit` in place of `u32`
/// let state = OwnedState::<u32>::create_temporary().expect("Failed to create state");
/// state.set(&42).expect("Failed to set state data");
///
/// assert!(can_read(state));
/// ```
///
/// Another use case is reading the change stamp of a state without knowing what the actual data looks like, but that is
/// already implemented in [`OwnedState::change_stamp`](crate::state::OwnedState::change_stamp).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OpaqueData {
    _private: (),
}

impl OpaqueData {
    /// Creates a new [`OpaqueData`]
    pub(crate) const fn new() -> Self {
        Self { _private: () }
    }
}

/// The change stamp of a state
///
/// This is `0` when the state is created and is increased by `1` on every update to the state.
///
/// Several methods on other types deal with change stamps. See their documentations for details:
/// - [`OwnedState::change_stamp`](crate::state::OwnedState::change_stamp)
/// - [`OwnedState::query`](crate::state::OwnedState::query) and
///   [`OwnedState::query_boxed`](crate::state::OwnedState::query_boxed)
/// - [`OwnedState::update`](crate::state::OwnedState::update)
/// - [`DataAccessor::change_stamp`](crate::subscribe::DataAccessor::change_stamp)
/// - [`DataAccessor::query`](crate::subscribe::DataAccessor::query) and
///   [`DataAccessor::query_boxed`](crate::subscribe::DataAccessor::query_boxed)
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChangeStamp(u32);

impl ChangeStamp {
    /// Creates a new [`ChangeStamp`] with the given value
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the initial change stamp of every state, which is `0`:
    ///
    /// ```
    /// # use wnf::ChangeStamp;
    /// #
    /// assert_eq!(ChangeStamp::initial().value(), 0);
    /// ```
    pub const fn initial() -> Self {
        Self(0)
    }

    /// Returns the inner value
    pub const fn value(self) -> u32 {
        self.0
    }

    /// Returns a mutable raw pointer to the inner value for use in FFI
    pub(crate) fn as_mut_ptr(&mut self) -> *mut u32 {
        &mut self.0
    }
}

impl From<u32> for ChangeStamp {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<ChangeStamp> for u32 {
    fn from(ChangeStamp(value): ChangeStamp) -> Self {
        value
    }
}

impl Display for ChangeStamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq<u32> for ChangeStamp {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}

impl PartialEq<ChangeStamp> for u32 {
    fn eq(&self, other: &ChangeStamp) -> bool {
        *self == other.0
    }
}

/// State data of type `T` together with a change stamp
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct StampedData<T> {
    data: T,
    change_stamp: ChangeStamp,
}

impl<T> StampedData<T> {
    /// Creates a new [`StampedData`] from the given data and change stamp
    pub fn from_data_change_stamp(data: T, change_stamp: impl Into<ChangeStamp>) -> Self {
        Self {
            data,
            change_stamp: change_stamp.into(),
        }
    }

    /// Moves the contained data and change stamp out of this [`StampedData`]
    pub fn into_data_change_stamp(self) -> (T, ChangeStamp) {
        (self.data, self.change_stamp)
    }

    /// Moves the contained data out of this [`StampedData`], discarding the change stamp
    pub fn into_data(self) -> T {
        self.data
    }

    /// Returns a reference to the data contained in this [`StampedData`]
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Returns the change stamp contained in this [`StampedData`]
    pub const fn change_stamp(&self) -> ChangeStamp {
        self.change_stamp
    }

    /// Returns a new [`StampedData`] obtained by applying a given closure to the contained data
    pub fn map<U, F>(self, op: F) -> StampedData<U>
    where
        F: FnOnce(T) -> U,
    {
        StampedData {
            data: op(self.data),
            change_stamp: self.change_stamp,
        }
    }
}

impl<T> From<(T, ChangeStamp)> for StampedData<T> {
    fn from((data, change_stamp): (T, ChangeStamp)) -> Self {
        Self { data, change_stamp }
    }
}

impl<T> From<StampedData<T>> for (T, ChangeStamp) {
    fn from(stamped_data: StampedData<T>) -> Self {
        (stamped_data.data, stamped_data.change_stamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_stamp_display() {
        assert_eq!(format!("{}", ChangeStamp::from(42)), "42");
    }

    #[test]
    fn stamped_data_map() {
        assert_eq!(
            StampedData::from_data_change_stamp(42, 1).map(|x| x.to_string()),
            StampedData::from_data_change_stamp(String::from("42"), 1)
        );
    }
}
