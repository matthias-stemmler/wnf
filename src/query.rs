//! Methods for querying state data

use std::{io, ptr};

use tracing::debug;
use windows::Win32::Foundation::STATUS_BUFFER_TOO_SMALL;

use crate::data::{ChangeStamp, OpaqueData, StampedData};
use crate::ntapi;
use crate::read::Read;
use crate::state::{BorrowedState, OwnedState, RawState};

impl<T> OwnedState<T>
where
    T: Read<T>,
{
    /// Queries the data of this [`OwnedState<T>`]
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`get_boxed`](OwnedState::get_boxed) method.
    ///
    /// This returns the data of the state without a change stamp. In order to query both the data and the change
    /// stamp, use the [`query`](OwnedState::query) method.
    ///
    /// # Errors
    /// Returns an error if querying fails, including the case that the queries data is not a valid `T`
    pub fn get(&self) -> io::Result<T> {
        self.raw.get()
    }

    /// Queries the data of this [`OwnedState<T>`] together with its change stamp
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`query_boxed`](OwnedState::query_boxed) method.
    ///
    /// This returns the data of the state together with its change stamp as a [`StampedData<T>`]. In order to
    /// only query the data, use the [`get`](OwnedState::get) method.
    ///
    /// # Errors
    /// Returns an error if querying fails, including the case that the queries data is not a valid `T`
    pub fn query(&self) -> io::Result<StampedData<T>> {
        self.raw.query()
    }
}

impl<T> OwnedState<T>
where
    T: Read<Box<T>> + ?Sized,
{
    /// Queries the data of this [`OwnedState<T>`] as a box
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`get`](OwnedState::get) method.
    ///
    /// This returns the data of the state without a change stamp. In order to query both the data and the change
    /// stamp, use the [`query_boxed`](OwnedState::query_boxed) method.
    ///
    /// # Errors
    /// Returns an error if querying fails, including the case that the queries data is not a valid `T`
    pub fn get_boxed(&self) -> io::Result<Box<T>> {
        self.raw.get_boxed()
    }

    /// Queries the data of this [`OwnedState<T>`] as a box together with its change stamp
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`query`](OwnedState::query) method.
    ///
    /// This returns the data of the state together with its change stamp as a [`StampedData<Box<T>>`]. In order
    /// to only query the data, use the [`get_boxed`](OwnedState::get_boxed) method.
    ///
    /// # Errors
    /// Returns an error if querying fails, including the case that the queries data is not a valid `T`
    pub fn query_boxed(&self) -> io::Result<StampedData<Box<T>>> {
        self.raw.query_boxed()
    }
}

impl<T> OwnedState<T>
where
    T: ?Sized,
{
    /// Queries the change stamp of this [`OwnedState<T>`]
    ///
    /// # Errors
    /// Returns an error if querying the change stamp fails
    pub fn change_stamp(&self) -> io::Result<ChangeStamp> {
        self.raw.change_stamp()
    }
}

impl<T> BorrowedState<'_, T>
where
    T: Read<T>,
{
    /// Queries the data of this [`BorrowedState<'a, T>`]
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`get_boxed`](BorrowedState::get_boxed) method.
    ///
    /// This returns the data of the state without a change stamp. In order to query both the data and the change
    /// stamp, use the [`query`](BorrowedState::query) method.
    ///
    /// # Errors
    /// Returns an error if querying fails, including the case that the queries data is not a valid `T`
    pub fn get(self) -> io::Result<T> {
        self.raw.get()
    }

    /// Queries the data of this [`BorrowedState<'a, T>`] together with its change stamp
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`query_boxed`](BorrowedState::query_boxed) method.
    ///
    /// This returns the data of the state together with its change stamp as a [`StampedData<T>`]. In order to
    /// only query the data, use the [`get`](BorrowedState::get) method.
    ///
    /// # Errors
    /// Returns an error if querying fails, including the case that the queries data is not a valid `T`
    pub fn query(self) -> io::Result<StampedData<T>> {
        self.raw.query()
    }
}

impl<T> BorrowedState<'_, T>
where
    T: Read<Box<T>> + ?Sized,
{
    /// Queries the data of this [`BorrowedState<'a, T>`] as a box
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`get`](BorrowedState::get) method.
    ///
    /// This returns the data of the state without a change stamp. In order to query both the data and the change
    /// stamp, use the [`query_boxed`](BorrowedState::query_boxed) method.
    ///
    /// # Errors
    /// Returns an error if querying fails, including the case that the queries data is not a valid `T`
    pub fn get_boxed(self) -> io::Result<Box<T>> {
        self.raw.get_boxed()
    }

    /// Queries the data of this [`BorrowedState<'a, T>`] as a box together with its change stamp
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`query`](BorrowedState::query) method.
    ///
    /// This returns the data of the state together with its change stamp as a [`StampedData<Box<T>>`]. In order
    /// to only query the data, use the [`get_boxed`](BorrowedState::get_boxed) method.
    ///
    /// # Errors
    /// Returns an error if querying fails, including the case that the queries data is not a valid `T`
    pub fn query_boxed(self) -> io::Result<StampedData<Box<T>>> {
        self.raw.query_boxed()
    }
}

impl<T> BorrowedState<'_, T>
where
    T: ?Sized,
{
    /// Queries the change stamp of this [`BorrowedState<'a, T>`]
    ///
    /// # Errors
    /// Returns an error if querying the change stamp fails
    pub fn change_stamp(self) -> io::Result<ChangeStamp> {
        self.raw.change_stamp()
    }
}

impl<T> RawState<T>
where
    T: Read<T>,
{
    /// Queries the data of this [`RawState<T>`]
    fn get(self) -> io::Result<T> {
        self.query().map(StampedData::into_data)
    }

    /// Queries the data of this [`RawState<T>`] together with its change stamp
    fn query(self) -> io::Result<StampedData<T>> {
        self.query_as()
    }
}

impl<T> RawState<T>
where
    T: Read<Box<T>> + ?Sized,
{
    /// Queries the data of this [`RawState<T>`] as a box
    fn get_boxed(self) -> io::Result<Box<T>> {
        self.query_boxed().map(StampedData::into_data)
    }

    /// Queries the data of this [`RawState<T>`] as a box together with its change stamp
    fn query_boxed(self) -> io::Result<StampedData<Box<T>>> {
        self.query_as()
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    /// Queries the change stamp of this [`RawState<T>`]
    pub(crate) fn change_stamp(self) -> io::Result<ChangeStamp> {
        Ok(self.cast::<OpaqueData>().query()?.change_stamp())
    }

    /// Queries the data of this [`RawState<T>`] as a value of type `D`
    ///
    /// If `T: Sized`, then `D` can be either `T` or `Box<T>`.
    /// If `T: !Sized`, then `D` must be `Box<T>`.
    pub(crate) fn query_as<D>(self) -> io::Result<StampedData<D>>
    where
        T: Read<D>,
    {
        let reader = |ptr, size| {
            let mut change_stamp = ChangeStamp::default();
            let mut read_size = size as u32;

            // SAFETY:
            // - The pointer in the first argument points to a valid `u64` because it comes from a live reference
            // - The pointer in the second argument is either a null pointer or points to a valid `GUID` by the
            //   guarantees of `TypeId::as_ptr`
            // - The pointer in the fourth argument is valid for writes of `u32` because it comes from a live mutable
            //   reference
            // - The pointer in the fifth argument is valid for writes of `read_size` by the precondition of `reader`
            //   (see `Read::from_reader`) and `read_size == size`
            // - The pointer in the sixth argument points to a valid `u32` because it comes from a live reference
            // - The pointer in the sixth argument is valid for writes of `u32` because it comes from a live mutable
            //   reference
            let result = unsafe {
                ntapi::NtQueryWnfStateData(
                    &self.state_name.opaque_value(),
                    self.type_id.as_ptr(),
                    ptr::null(),
                    change_stamp.as_mut_ptr(),
                    ptr,
                    &mut read_size,
                )
            };

            if result.is_err() && (result != STATUS_BUFFER_TOO_SMALL || read_size as usize <= size) {
                debug!(
                     target: ntapi::TRACING_TARGET,
                     ?result,
                     input.state_name = %self.state_name,
                     input.type_id = %self.type_id,
                     "NtQueryWnfStateData",
                );

                Err(io::Error::from_raw_os_error(result.0))
            } else {
                // Here we know that either of the following conditions holds:
                // a) `result.is_ok()`
                // b) `result == STATUS_BUFFER_TOO_SMALL && read_size as usize > size`
                debug!(
                    target: ntapi::TRACING_TARGET,
                    ?result,
                    input.state_name = %self.state_name,
                    input.type_id = %self.type_id,
                    output.change_stamp = %change_stamp,
                    output.buffer_size = read_size,
                    "NtQueryWnfStateData",
                );

                Ok((read_size as usize, change_stamp))
            }
        };

        // SAFETY:
        // When `reader(ptr, size)` returns `Ok((read_size, _))` with `read_size <= size`,
        // - then condition a) (see above) holds,
        // - hence the call to `NtQueryWnfStateData` succeeded,
        // - hence by the assumption on `NtQueryWnfStateData`, the memory range of size `read_size` starting at `ptr` is
        //   initialized,
        // so the safety condition of `T::from_reader` is satisfied
        let result = unsafe { T::from_reader(reader) };

        Ok(result?.into())
    }
}
