//! Methods for applying a transformation to state data

#![deny(unsafe_code)]

use std::borrow::Borrow;
use std::convert::Infallible;
use std::error::Error;
use std::io;
use std::io::ErrorKind;

use crate::bytes::NoUninit;
use crate::read::Read;
use crate::state::{BorrowedState, OwnedState, RawState};

impl<T> OwnedState<T>
where
    T: Read<T> + NoUninit,
{
    /// Applies a transformation to the data of this state
    ///
    /// This essentially queries the state data, applies the given transformation closure and then updates the state
    /// data with the transformed value. However, it tries to do so in a loop using change stamps to ensure that no
    /// concurrent update happens between querying and updating the state data. This means that the given closure may be
    /// called multiple times. Note that it does *not* reliably avoid concurrent updates while the actual update is
    /// happening. If another concurrent update makes the size of the state data exceed the internal capacity of the
    /// state (causing a reallocation), it may happen that this update does not have the desired effect on the state
    /// data.
    ///
    /// The closure receives an owned `T` on the stack, requiring `T: Sized`. In order to receive a `Box<T>` for
    /// `T: ?Sized`, use the [`apply_boxed`](OwnedState::apply_boxed) method.
    ///
    /// For fallible transformations, i.e. when the transformation closure returns a [`Result<D, E>`], use the
    /// [`try_apply`](OwnedState::try_apply) method.
    ///
    /// The return value is the value with which the state was ultimately updated, i.e. the return value of the last
    /// call to the given closure.
    ///
    /// For example, to increment the value of a state by one and return the incremented value:
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use std::io;
    ///
    /// use wnf::{AsState, OwnedState};
    ///
    /// fn increment<S>(state: S) -> io::Result<u32>
    /// where
    ///     S: AsState<Data = u32>,
    /// {
    ///     state.as_state().apply(|value| value + 1)
    /// }
    ///
    /// let state = OwnedState::create_temporary()?;
    /// state.set(&42)?;
    ///
    /// let new_data = increment(&state)?;
    /// assert_eq!(new_data, 43);
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    /// Returns an error if querying or updating fails
    pub fn apply<D, F>(&self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        F: FnMut(T) -> D,
    {
        self.raw.apply(transform)
    }

    /// Applies a fallible transformation to the data of this state
    ///
    /// This essentially queries the state data, applies the given transformation closure and then (in case it succeeds)
    /// updates the state data with the transformed value. However, it tries to do so in a loop using change stamps
    /// to ensure that no concurrent update happens between querying and updating the state data. This means that the
    /// given closure may be called multiple times. Note that it does *not* reliably avoid concurrent updates while the
    /// actual update is happening. If another concurrent update makes the size of the state data exceed the
    /// internal capacity of the state (causing a reallocation), it may happen that this update does not have the
    /// desired effect on the state data.
    ///
    /// The closure receives an owned `T` on the stack, requiring `T: Sized`. In order to receive a `Box<T>` for
    /// `T: ?Sized`, use the [`apply_boxed`](OwnedState::apply_boxed) method.
    ///
    /// For infallible transformations, i.e. when the transformation closure returns a `D` rather than a [`Result<D,
    /// E>`], use the [`apply`](OwnedState::apply) method.
    ///
    /// The return value is the value with which the state was ultimately updated, i.e. the return value of the last
    /// call to the given closure.
    ///
    /// For example, to increment the value of a state by one, unless a maximum is reached, and return the incremented
    /// value:
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use std::io;
    ///
    /// use wnf::{AsState, OwnedState};
    ///
    /// fn try_increment<S>(state: S, max: u32) -> io::Result<u32>
    /// where
    ///     S: AsState<Data = u32>,
    /// {
    ///     state.as_state().try_apply(|value| {
    ///         if value < max {
    ///             Ok(value + 1)
    ///         } else {
    ///             Err("maximum reached")
    ///         }
    ///     })
    /// }
    ///
    /// let state = OwnedState::create_temporary()?;
    /// state.set(&42)?;
    ///
    /// let new_data = try_increment(&state, 43)?;
    /// assert_eq!(new_data, 43);
    ///
    /// let result = try_increment(&state, 43);
    /// assert!(result.is_err());
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    /// Returns an error if querying or updating fails, or if the transformation closure returns an error
    pub fn try_apply<D, E, F>(&self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(T) -> Result<D, E>,
    {
        self.raw.try_apply(transform)
    }
}

impl<T> OwnedState<T>
where
    T: Read<Box<T>> + NoUninit + ?Sized,
{
    /// Applies a transformation to the data of this state as a box
    ///
    /// This essentially queries the state data, applies the given transformation closure and then updates the state
    /// data with the transformed value. However, it tries to do so in a loop using change stamps to ensure that no
    /// concurrent update happens between querying and updating the state data. This means that the given closure may
    /// be called multiple times. Note that it does *not* reliably avoid concurrent updates while the actual update is
    /// happening. If another concurrent update makes the size of the state data exceed the internal capacity of the
    /// state (causing a reallocation), it may happen that this update does not have the desired effect on the state
    /// data.
    ///
    /// The closure receives a [`Box<T>`]. In order to receive an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`apply`](OwnedState::apply) method.
    ///
    /// For fallible transformations, i.e. when the transformation closure returns a [`Result<D, E>`], use the
    /// [`try_apply_boxed`](OwnedState::try_apply_boxed) method.
    ///
    /// The return value is the value with which the state was ultimately updated, i.e. the return value of the last
    /// call to the given closure.
    ///
    /// For example, to extend a slice by one element and return the extended slice as a [`Vec<u32>`]:
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use std::io;
    ///
    /// use wnf::{AsState, OwnedState};
    ///
    /// fn extend<S>(state: S) -> io::Result<Vec<u32>>
    /// where
    ///     S: AsState<Data = [u32]>,
    /// {
    ///     state.as_state().apply_boxed(|slice| {
    ///         let mut vec = slice.into_vec();
    ///         vec.push(42);
    ///         vec
    ///     })
    /// }
    ///
    /// let state = OwnedState::<[u32]>::create_temporary()?;
    /// state.set(&[1, 2, 3])?;
    ///
    /// let new_data = extend(&state)?;
    /// assert_eq!(*new_data, [1, 2, 3, 42]);
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    /// Returns an error if querying or updating fails
    pub fn apply_boxed<D, F>(&self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> D,
    {
        self.raw.apply_boxed(transform)
    }

    /// Applies a fallible transformation to the data of this state as a box
    ///
    /// This essentially queries the state data, applies the given transformation closure and then (in case it succeeds)
    /// updates the state data with the transformed value. However, it tries to do so in a loop using change stamps
    /// to ensure that no concurrent update happens between querying and updating the state data. This means that the
    /// given closure may be called multiple times. Note that it does *not* reliably avoid concurrent updates while the
    /// actual update is happening. If another concurrent update makes the size of the state data exceed the internal
    /// capacity of the state (causing a reallocation), it may happen that this update does not have the desired effect
    /// on the state data.
    ///
    /// The closure receives a [`Box<T>`]. In order to receive an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`try_apply`](OwnedState::try_apply) method.
    ///
    /// For infallible transformations, i.e. when the transformation closure returns a `D` rather than a [`Result<D,
    /// E>`], use the [`apply_boxed`](OwnedState::apply_boxed) method.
    ///
    /// The return value is the value with which the state was ultimately updated, i.e. the return value of the last
    /// call to the given closure.
    ///
    /// For example, to extend a slice by one element, unless a maximum length is reached, and return the extended
    /// slice as a [`Vec<u32>`]:
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use std::io;
    ///
    /// use wnf::{AsState, OwnedState};
    ///
    /// fn try_extend<S>(state: S, max_len: usize) -> io::Result<Vec<u32>>
    /// where
    ///     S: AsState<Data = [u32]>,
    /// {
    ///     state.as_state().try_apply_boxed(|slice| {
    ///         if slice.len() < max_len {
    ///             let mut vec = slice.into_vec();
    ///             vec.push(42);
    ///             Ok(vec)
    ///         } else {
    ///             Err("maximum length reached")
    ///         }
    ///     })
    /// }
    ///
    /// let state = OwnedState::<[u32]>::create_temporary()?;
    /// state.set(&[1, 2, 3])?;
    ///
    /// let new_data = try_extend(&state, 4)?;
    /// assert_eq!(*new_data, [1, 2, 3, 42]);
    ///
    /// let result = try_extend(&state, 4);
    /// assert!(result.is_err());
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    /// Returns an error if querying or updating fails
    pub fn try_apply_boxed<D, E, F>(&self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(Box<T>) -> Result<D, E>,
    {
        self.raw.try_apply_boxed(transform)
    }
}

impl<T> BorrowedState<'_, T>
where
    T: Read<T> + NoUninit,
{
    /// Applies a transformation to the data of this state
    ///
    /// See [`OwnedState::apply`]
    pub fn apply<D, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        F: FnMut(T) -> D,
    {
        self.raw.apply(transform)
    }

    /// Applies a fallible transformation to the data of this state
    ///
    /// See [`OwnedState::try_apply`]
    pub fn try_apply<D, E, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(T) -> Result<D, E>,
    {
        self.raw.try_apply(transform)
    }
}

impl<T> BorrowedState<'_, T>
where
    T: Read<Box<T>> + NoUninit + ?Sized,
{
    /// Applies a transformation to the data of this state as a box
    ///
    /// See [`OwnedState::apply_boxed`]
    pub fn apply_boxed<D, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> D,
    {
        self.raw.apply_boxed(transform)
    }

    /// Applies a fallible transformation to the data of this state as a box
    ///
    /// See [`OwnedState::try_apply_boxed`]
    pub fn try_apply_boxed<D, E, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(Box<T>) -> Result<D, E>,
    {
        self.raw.try_apply_boxed(transform)
    }
}

impl<T> RawState<T>
where
    T: Read<T> + NoUninit,
{
    /// Applies a transformation to the data of this state
    fn apply<D, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        F: FnMut(T) -> D,
    {
        self.apply_as(transform)
    }

    /// Applies a fallible transformation to the data of this state
    fn try_apply<D, E, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(T) -> Result<D, E>,
    {
        self.try_apply_as(transform)
    }
}

impl<T> RawState<T>
where
    T: Read<Box<T>> + NoUninit + ?Sized,
{
    /// Applies a transformation to the data of this state as a box
    fn apply_boxed<D, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> D,
    {
        self.apply_as(transform)
    }

    /// Applies a fallible transformation to the data of this state as a box
    fn try_apply_boxed<D, E, F>(self, transform: F) -> io::Result<D>
    where
        D: Borrow<T>,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(Box<T>) -> Result<D, E>,
    {
        self.try_apply_as(transform)
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    /// Applies a transformation to the data of this state, passing a value of type `D` to it
    ///
    /// If `T: Sized`, then `D` can be either `T` or `Box<T>`.
    /// If `T: !Sized`, then `D` must be `Box<T>`.
    pub(crate) fn apply_as<ReadInto, WriteFrom, F>(self, mut transform: F) -> io::Result<WriteFrom>
    where
        WriteFrom: Borrow<T>,
        T: Read<ReadInto> + NoUninit,
        F: FnMut(ReadInto) -> WriteFrom,
    {
        self.try_apply_as(|data| Ok::<_, Infallible>(transform(data)))
    }

    /// Applies a fallible transformation to the data of this state, passing a value of type `D` to it
    ///
    /// If `T: Sized`, then `D` can be either `T` or `Box<T>`.
    /// If `T: !Sized`, then `D` must be `Box<T>`.
    fn try_apply_as<ReadInto, WriteFrom, E, F>(self, mut transform: F) -> io::Result<WriteFrom>
    where
        WriteFrom: Borrow<T>,
        T: Read<ReadInto> + NoUninit,
        E: Into<Box<dyn Error + Send + Sync>>,
        F: FnMut(ReadInto) -> Result<WriteFrom, E>,
    {
        let result = loop {
            let (data, change_stamp) = self.query_as()?.into_data_change_stamp();
            let result = transform(data).map_err(|err| io::Error::new(ErrorKind::Other, err))?;
            if self.update(result.borrow(), change_stamp)? {
                break result;
            }
        };

        Ok(result)
    }
}
