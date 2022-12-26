//! Methods for updating state data while returning the previous value
//!
//! This module only adds inherent impls to [`OwnedState<T>`] and [`BorrowedState<'_, T>`](BorrowedState).

#![deny(unsafe_code)]

use std::io;

use crate::bytes::NoUninit;
use crate::read::Read;
use crate::state::{BorrowedState, OwnedState, RawState};

impl<T> OwnedState<T>
where
    T: Read<T> + NoUninit,
{
    /// Replaces the data of this state, returning the previous value
    ///
    /// This essentially queries the state data, updates it with the given value and returns the previous value.
    /// However, it tries to do so in a loop using change stamps to ensure that no concurrent update happens between
    /// querying and updating the data. Note that it does *not* reliably avoid concurrent updates while the actual
    /// update is happening. If another concurrent update makes the size of the state data exceed the internal
    /// capacity of the state (causing a reallocation), it may happen that this update does not have the desired
    /// effect on the state data.
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`replace_boxed`](OwnedState::replace_boxed) method.
    ///
    /// For example, to set the value of a state to zero while returning the previous value:
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use std::io;
    ///
    /// use wnf::{AsState, OwnedState};
    ///
    /// fn reset<S>(state: S) -> io::Result<u32>
    /// where
    ///     S: AsState<Data = u32>,
    /// {
    ///     state.as_state().replace(&0)
    /// }
    ///
    /// let state = OwnedState::create_temporary()?;
    /// state.set(&42)?;
    ///
    /// let previous_value = reset(&state)?;
    /// assert_eq!(previous_value, 42);
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    /// Returns an error if querying or updating fails
    pub fn replace(&self, new_value: &T) -> io::Result<T> {
        self.raw.replace(new_value)
    }
}

impl<T> OwnedState<T>
where
    T: Read<Box<T>> + NoUninit + ?Sized,
{
    /// Replaces the data of this state, returning the previous value as a box
    ///
    /// This essentially queries the state data, updates it with the given value and returns the previous value.
    /// However, it tries to do so in a loop using change stamps to ensure that no concurrent update happens between
    /// querying and updating the data. Note that it does *not* reliably avoid concurrent updates while the actual
    /// update is happening. If another concurrent update makes the size of the state data exceed the internal
    /// capacity of the state (causing a reallocation), it may happen that this update does not have the desired
    /// effect on the state data.
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`replace`](OwnedState::replace) method.
    ///
    /// For example, to make a slice empty while returning the previous (boxed) slice:
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use std::io;
    ///
    /// use wnf::{AsState, OwnedState};
    ///
    /// fn clear<S>(state: S) -> io::Result<Box<[u32]>>
    /// where
    ///     S: AsState<Data = [u32]>,
    /// {
    ///     state.as_state().replace_boxed(&[])
    /// }
    ///
    /// let state = OwnedState::<[u32]>::create_temporary()?;
    /// state.set(&[1, 2, 3])?;
    ///
    /// let previous_value = clear(&state)?;
    /// assert_eq!(*previous_value, [1, 2, 3]);
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    /// Returns an error if querying or updating fails
    pub fn replace_boxed(&self, new_value: &T) -> io::Result<Box<T>> {
        self.raw.replace_boxed(new_value)
    }
}

impl<T> BorrowedState<'_, T>
where
    T: Read<T> + NoUninit,
{
    /// Replaces the data of this state, returning the previous value
    ///
    /// See [`OwnedState::replace`]
    pub fn replace(self, new_value: &T) -> io::Result<T> {
        self.raw.replace(new_value)
    }
}

impl<T> BorrowedState<'_, T>
where
    T: Read<Box<T>> + NoUninit + ?Sized,
{
    /// Replaces the data of this state, returning the previous value as a box
    ///
    /// See [`OwnedState::replace_boxed`]
    pub fn replace_boxed(self, new_value: &T) -> io::Result<Box<T>> {
        self.raw.replace_boxed(new_value)
    }
}

impl<T> RawState<T>
where
    T: Read<T> + NoUninit,
{
    /// Replaces the data of this state, returning the previous value
    fn replace(self, new_value: &T) -> io::Result<T> {
        self.replace_as(new_value)
    }
}

impl<T> RawState<T>
where
    T: Read<Box<T>> + NoUninit + ?Sized,
{
    /// Replaces the data of this state, returning the previous value as a box
    fn replace_boxed(self, new_value: &T) -> io::Result<Box<T>> {
        self.replace_as(new_value)
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    /// Replaces the data of this state, returning the previous value as a value of type `D`
    ///
    /// If `T: Sized`, then `D` can be either `T` or `Box<T>`.
    /// If `T: !Sized`, then `D` must be `Box<T>`.
    fn replace_as<D>(self, new_value: &T) -> io::Result<D>
    where
        T: Read<D> + NoUninit,
    {
        let mut old_value = None;

        self.apply_as(|value| {
            old_value = Some(value);
            new_value
        })?;

        Ok(old_value.unwrap())
    }
}
