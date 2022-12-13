//! Types dealing with the data of a state

#![deny(unsafe_code)]

use std::borrow::{Borrow, BorrowMut};
use std::fmt;
use std::fmt::{Display, Formatter};

/// Placeholder for state data whose content is irrelevant
///
/// This type can be "read" from any state regardless of the size of the state data. It doesn't contain the actual data
/// but just their size, which can be obtained via the [`OpaqueData::size`] method. This is useful on different
/// occasions, for instance:
/// - If you want to query the size of state data without querying the actual data:
/// ```
/// # use std::io;
/// # use wnf::{AsState, OwnedState, OpaqueData};
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let state = OwnedState::<u32>::create_temporary()?;
/// state.set(&42)?;
///
/// assert_eq!(state.as_state().cast::<OpaqueData>().get()?.size(), 4);
///
/// // Less efficient alternative:
/// assert_eq!(state.as_state().cast::<[u8]>().get_boxed()?.len(), 4);
/// # Ok(()) }
/// ```
/// - If you want to check if a state can be read without querying its actual data:
/// ```
/// # use std::io;
/// # use wnf::{OwnedState, OpaqueData};
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Here we could have used any type `T: NoUninit` in place of `u32`
/// let state = OwnedState::<u32>::create_temporary()?;
/// state.set(&42)?;
///
/// let can_read = state.cast::<OpaqueData>().get().is_ok();
///
/// assert!(can_read);
/// # Ok(()) }
/// ```
///
/// Another use case is reading the change stamp of a state without knowing what the actual data look like, but that is
/// already implemented in [`OwnedState::change_stamp`](crate::state::OwnedState::change_stamp).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OpaqueData {
    size: usize,
}

impl OpaqueData {
    /// Creates a new [`OpaqueData`]
    pub(crate) const fn new(size: usize) -> Self {
        Self { size }
    }

    /// Returns the size in bytes of this [`OpaqueData`]
    pub const fn size(self) -> usize {
        self.size
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

impl AsRef<u32> for ChangeStamp {
    fn as_ref(&self) -> &u32 {
        &self.0
    }
}

impl AsMut<u32> for ChangeStamp {
    fn as_mut(&mut self) -> &mut u32 {
        &mut self.0
    }
}

impl Borrow<u32> for ChangeStamp {
    fn borrow(&self) -> &u32 {
        &self.0
    }
}

impl BorrowMut<u32> for ChangeStamp {
    fn borrow_mut(&mut self) -> &mut u32 {
        &mut self.0
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
    pub const fn data(&self) -> &T {
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

impl<T> AsRef<T> for StampedData<T> {
    fn as_ref(&self) -> &T {
        &self.data
    }
}

impl<T> AsMut<T> for StampedData<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_stamp_display() {
        assert_eq!(ChangeStamp::from(42).to_string(), "42");
    }

    #[test]
    fn stamped_data_map() {
        assert_eq!(
            StampedData::from_data_change_stamp(42, 1).map(|x| x.to_string()),
            StampedData::from_data_change_stamp(String::from("42"), 1)
        );
    }
}
