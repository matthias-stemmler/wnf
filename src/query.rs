use std::ffi::c_void;
use std::{io, ptr};

use tracing::debug;
use windows::Win32::Foundation::STATUS_BUFFER_TOO_SMALL;

use crate::data::{WnfChangeStamp, WnfStampedData};
use crate::ntdll::NTDLL_TARGET;
use crate::read::WnfRead;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::type_id::TypeId;
use crate::{ntdll_sys, WnfOpaqueData, WnfStateName};

impl<T> OwnedWnfState<T>
where
    T: WnfRead<T>,
{
    pub fn get(&self) -> io::Result<T> {
        self.raw.get()
    }

    pub fn query(&self) -> io::Result<WnfStampedData<T>> {
        self.raw.query()
    }
}

impl<T> OwnedWnfState<T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    pub fn get_boxed(&self) -> io::Result<Box<T>> {
        self.raw.get_boxed()
    }

    pub fn query_boxed(&self) -> io::Result<WnfStampedData<Box<T>>> {
        self.raw.query_boxed()
    }
}

impl<T> OwnedWnfState<T>
where
    T: ?Sized,
{
    pub fn change_stamp(&self) -> io::Result<WnfChangeStamp> {
        self.raw.change_stamp()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead<T>,
{
    pub fn get(self) -> io::Result<T> {
        self.raw.get()
    }

    pub fn query(self) -> io::Result<WnfStampedData<T>> {
        self.raw.query()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    pub fn get_boxed(self) -> io::Result<Box<T>> {
        self.raw.get_boxed()
    }

    pub fn query_boxed(self) -> io::Result<WnfStampedData<Box<T>>> {
        self.raw.query_boxed()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: ?Sized,
{
    pub fn change_stamp(self) -> io::Result<WnfChangeStamp> {
        self.raw.change_stamp()
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<T>,
{
    pub fn get(self) -> io::Result<T> {
        self.query().map(WnfStampedData::into_data)
    }

    pub fn query(self) -> io::Result<WnfStampedData<T>> {
        self.query_as()
    }
}

impl<T> RawWnfState<T>
where
    T: WnfRead<Box<T>> + ?Sized,
{
    pub fn get_boxed(self) -> io::Result<Box<T>> {
        self.query_boxed().map(WnfStampedData::into_data)
    }

    pub fn query_boxed(self) -> io::Result<WnfStampedData<Box<T>>> {
        self.query_as()
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    pub fn change_stamp(self) -> io::Result<WnfChangeStamp> {
        Ok(self.cast::<WnfOpaqueData>().query()?.change_stamp())
    }

    pub(crate) fn query_as<D>(self) -> io::Result<WnfStampedData<D>>
    where
        T: WnfRead<D>,
    {
        let data =
            unsafe { T::from_reader(|buffer, buffer_size| query(self.state_name, self.type_id, buffer, buffer_size)) }?;
        Ok(data.into())
    }
}

unsafe fn query(
    state_name: WnfStateName,
    type_id: TypeId,
    buffer: *mut c_void,
    buffer_size: usize,
) -> io::Result<(usize, WnfChangeStamp)> {
    let mut change_stamp = WnfChangeStamp::default();
    let mut size = buffer_size as u32;

    let result = ntdll_sys::ZwQueryWnfStateData(
        &state_name.opaque_value(),
        type_id.as_ptr(),
        ptr::null(),
        change_stamp.as_mut_ptr(),
        buffer,
        &mut size,
    );

    if result.is_err() && (result != STATUS_BUFFER_TOO_SMALL || size as usize <= buffer_size) {
        debug!(
             target: NTDLL_TARGET,
             ?result,
             input.state_name = %state_name,
             input.type_id = %type_id,
             "ZwQueryWnfStateData",
        );

        Err(io::Error::from_raw_os_error(result.0))
    } else {
        debug!(
            target: NTDLL_TARGET,
            ?result,
            input.state_name = %state_name,
            input.type_id = %type_id,
            output.change_stamp = %change_stamp,
            output.buffer_size = size,
            "ZwQueryWnfStateData",
        );

        Ok((size as usize, change_stamp))
    }
}
