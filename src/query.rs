//! Methods for querying WNF state data

use std::{io, ptr};

use tracing::debug;
use windows::Win32::Foundation::STATUS_BUFFER_TOO_SMALL;

use crate::data::{WnfChangeStamp, WnfOpaqueData, WnfStampedData};
use crate::ntapi;
use crate::read::WnfRead;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};

impl<T> OwnedWnfState<T>
where
    T: WnfRead<T>,
{
    /// Queries the data of this [`OwnedWnfState<T>`]
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`get_boxed`](OwnedWnfState::get_boxed) method.
    ///
    /// This returns the data of the WNF state without a change stamp. In order to query both the data and the change
    /// stamp, use the [`query`](OwnedWnfState::query) method.
    pub fn get(&self) -> io::Result<T> {
        self.raw.get()
    }

    /// Queries the data of this [`OwnedWnfState<T>`] together with its change stamp
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`query_boxed`](OwnedWnfState::query_boxed) method.
    ///
    /// This returns the data of the WNF state together with its change stamp as a [`WnfStampedData<T>`]. In order to
    /// only query the data, use the [`get`](OwnedWnfState::get) method.
    pub fn query(&self) -> io::Result<WnfStampedData<T>> {
        self.raw.query()
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    /// Queries the data of this [`OwnedWnfState<T>`] as a box
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`get`](OwnedWnfState::get) method.
    ///
    /// This returns the data of the WNF state without a change stamp. In order to query both the data and the change
    /// stamp, use the [`query_boxed`](OwnedWnfState::query_boxed) method.
    pub fn get_boxed(&self) -> io::Result<Box<T>> {
        self.raw.get_boxed()
    }

    /// Queries the data of this [`OwnedWnfState<T>`] as a box together with its change stamp
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`query`](OwnedWnfState::query) method.
    ///
    /// This returns the data of the WNF state together with its change stamp as a [`WnfStampedData<Box<T>>`]. In order
    /// to only query the data, use the [`get_boxed`](OwnedWnfState::get_boxed) method.
    pub fn query_boxed(&self) -> io::Result<WnfStampedData<Box<T>>> {
        self.raw.query_boxed()
    }
}

impl<T> OwnedWnfState<T>
where
    T: ?Sized,
{
    /// Queries the change stamp of this [`OwnedWnfState<T>`]
    pub fn change_stamp(&self) -> io::Result<WnfChangeStamp> {
        self.raw.change_stamp()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead<T>,
{
    /// Queries the data of this [`BorrowedWnfState<'a, T>`]
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`get_boxed`](BorrowedWnfState::get_boxed) method.
    ///
    /// This returns the data of the WNF state without a change stamp. In order to query both the data and the change
    /// stamp, use the [`query`](BorrowedWnfState::query) method.
    pub fn get(self) -> io::Result<T> {
        self.raw.get()
    }

    /// Queries the data of this [`BorrowedWnfState<'a, T>`] together with its change stamp
    ///
    /// This produces an owned `T` on the stack and hence requires `T: Sized`. In order to produce a `Box<T>` for
    /// `T: ?Sized`, use the [`query_boxed`](BorrowedWnfState::query_boxed) method.
    ///
    /// This returns the data of the WNF state together with its change stamp as a [`WnfStampedData<T>`]. In order to
    /// only query the data, use the [`get`](BorrowedWnfState::get) method.
    pub fn query(self) -> io::Result<WnfStampedData<T>> {
        self.raw.query()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    /// Queries the data of this [`BorrowedWnfState<'a, T>`] as a box
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`get`](BorrowedWnfState::get) method.
    ///
    /// This returns the data of the WNF state without a change stamp. In order to query both the data and the change
    /// stamp, use the [`query_boxed`](BorrowedWnfState::query_boxed) method.
    pub fn get_boxed(self) -> io::Result<Box<T>> {
        self.raw.get_boxed()
    }

    /// Queries the data of this [`BorrowedWnfState<'a, T>`] as a box together with its change stamp
    ///
    /// This produces a [`Box<T>`]. In order to produce an owned `T` on the stack (requiring `T: Sized`), use the
    /// [`query`](BorrowedWnfState::query) method.
    ///
    /// This returns the data of the WNF state together with its change stamp as a [`WnfStampedData<Box<T>>`]. In order
    /// to only query the data, use the [`get_boxed`](BorrowedWnfState::get_boxed) method.
    pub fn query_boxed(self) -> io::Result<WnfStampedData<Box<T>>> {
        self.raw.query_boxed()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: ?Sized,
{
    /// Queries the change stamp of this [`BorrowedWnfState<'a, T>`]
    pub fn change_stamp(self) -> io::Result<WnfChangeStamp> {
        self.raw.change_stamp()
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<T>,
{
    /// Queries the data of this [`RawWnfState<T>`]
    fn get(self) -> io::Result<T> {
        self.query().map(WnfStampedData::into_data)
    }

    /// Queries the data of this [`RawWnfState<T>`] together with its change stamp
    fn query(self) -> io::Result<WnfStampedData<T>> {
        self.query_as()
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    /// Queries the data of this [`RawWnfState<T>`] as a box
    fn get_boxed(self) -> io::Result<Box<T>> {
        self.query_boxed().map(WnfStampedData::into_data)
    }

    /// Queries the data of this [`RawWnfState<T>`] as a box together with its change stamp
    fn query_boxed(self) -> io::Result<WnfStampedData<Box<T>>> {
        self.query_as()
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    /// Queries the change stamp of this [`RawWnfState<T>`]
    pub(crate) fn change_stamp(self) -> io::Result<WnfChangeStamp> {
        Ok(self.cast::<WnfOpaqueData>().query()?.change_stamp())
    }

    /// Queries the data of this [`RawWnfState<T>`] as a value of type `D`
    ///
    /// If `T: Sized`, then `D` can be either `T` or `Box<T>`.
    /// If `T: !Sized`, then `D` must be `Box<T>`.
    pub(crate) fn query_as<D>(self) -> io::Result<WnfStampedData<D>>
    where
        T: WnfRead<D>,
    {
        let reader = |ptr, size| {
            let mut change_stamp = WnfChangeStamp::default();
            let mut read_size = size as u32;

            // SAFETY:
            // - The pointer in the first argument points to a valid `u64` because it comes from a live reference
            // - The pointer in the second argument is either a null pointer or points to a valid `GUID` by the
            //   guarantees of `TypeId::as_ptr`
            // - The pointer in the fourth argument is valid for writes of `u32` because it comes from a live mutable
            //   reference
            // - The pointer in the fifth argument is valid for writes of `read_size` by the precondition of `reader`
            //   (see `WnfRead::from_reader`) and `read_size == size`
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
