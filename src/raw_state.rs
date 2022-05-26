use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::{mem, panic, ptr, slice};

use windows::core::GUID;
use windows::Win32::Foundation::{
    NTSTATUS, STATUS_BUFFER_TOO_SMALL, STATUS_OBJECT_NAME_NOT_FOUND, STATUS_SUCCESS, STATUS_WAIT_1,
};

use crate::bytes::{CheckedBitPattern, NoUninit};
use crate::data::WnfStateInfo;
use crate::error::{WnfApplyError, WnfDeleteError, WnfInfoError, WnfQueryError, WnfSubscribeError, WnfUpdateError};
use crate::subscription::{WnfSubscriptionContext, WnfSubscriptionHandle};
use crate::{
    ntdll_sys, SecurityDescriptor, WnfChangeStamp, WnfCreateError, WnfDataScope, WnfStampedData, WnfStateName,
    WnfStateNameLifetime,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct RawWnfState {
    state_name: WnfStateName,
}

impl RawWnfState {
    pub(crate) fn from_state_name(state_name: WnfStateName) -> Self {
        Self { state_name }
    }

    pub(crate) fn state_name(&self) -> WnfStateName {
        self.state_name
    }

    pub(crate) fn create_temporary() -> Result<Self, WnfCreateError> {
        let mut opaque_value = 0;
        // TODO Can we drop this or is it "borrowed" by the created WNF state?
        let security_descriptor = SecurityDescriptor::create_everyone_generic_all()?;

        unsafe {
            ntdll_sys::ZwCreateWnfStateName(
                &mut opaque_value,
                WnfStateNameLifetime::Temporary as u32,
                WnfDataScope::Machine as u32,
                0,
                ptr::null(),
                0x1000,
                security_descriptor.as_void_ptr(),
            )
        }
        .ok()?;

        Ok(Self::from_state_name(WnfStateName::from_opaque_value(opaque_value)))
    }

    pub(crate) fn delete(self) -> Result<(), WnfDeleteError> {
        Ok(unsafe { ntdll_sys::ZwDeleteWnfStateName(&self.state_name.opaque_value()) }.ok()?)
    }

    pub fn exists(&self) -> Result<bool, WnfInfoError> {
        Ok(self.info()?.is_some())
    }

    pub fn info(&self) -> Result<Option<WnfStateInfo>, WnfInfoError> {
        let mut change_stamp = WnfChangeStamp::default();
        let mut size = 0;

        let result = unsafe {
            ntdll_sys::ZwQueryWnfStateData(
                &self.state_name.opaque_value(),
                ptr::null(),
                ptr::null(),
                change_stamp.as_mut_ptr(),
                ptr::null_mut(),
                &mut size,
            )
        };

        Ok(if result == STATUS_OBJECT_NAME_NOT_FOUND {
            None
        } else {
            result.ok()?;
            Some(WnfStateInfo::from_size_change_stamp(size, change_stamp))
        })
    }
}

impl RawWnfState {
    pub fn get<T>(&self) -> Result<T, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.query().map(WnfStampedData::into_data)
    }

    pub fn get_slice<T>(&self) -> Result<Box<[T]>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.query_slice().map(WnfStampedData::into_data)
    }

    pub fn query<T>(&self) -> Result<WnfStampedData<T>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        let mut buffer = MaybeUninit::<T::Bits>::uninit();
        let (size, change_stamp) = unsafe { self.query_internal(buffer.as_mut_ptr(), mem::size_of::<T::Bits>())? };

        if size != mem::size_of::<T::Bits>() {
            return Err(WnfQueryError::WrongSize {
                expected: mem::size_of::<T::Bits>(),
                actual: size,
            });
        }

        let bits = unsafe { buffer.assume_init() };

        if T::is_valid_bit_pattern(&bits) {
            let data = unsafe { *(&bits as *const T::Bits as *const T) };
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(WnfQueryError::InvalidBitPattern)
        }
    }

    pub fn query_slice<T>(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        let mut buffer: Vec<T::Bits> = Vec::new();

        let (len, change_stamp) = loop {
            let (size, change_stamp) =
                unsafe { self.query_internal(buffer.as_mut_ptr(), buffer.capacity() * mem::size_of::<T::Bits>())? };

            if size == 0 {
                break (0, change_stamp);
            }

            if mem::size_of::<T::Bits>() == 0 {
                return Err(WnfQueryError::WrongSize {
                    expected: 0,
                    actual: size,
                });
            }

            if size % mem::size_of::<T::Bits>() != 0 {
                return Err(WnfQueryError::WrongSizeMultiple {
                    expected_modulus: mem::size_of::<T::Bits>(),
                    actual: size,
                });
            }

            let len = size / mem::size_of::<T::Bits>();
            if len > buffer.capacity() {
                buffer.reserve(len - buffer.capacity());
            } else {
                break (len, change_stamp);
            }
        };

        unsafe {
            buffer.set_len(len);
        }

        if buffer.iter().all(T::is_valid_bit_pattern) {
            let data = buffer.into_boxed_slice();
            let data = unsafe { Box::from_raw(Box::into_raw(data) as *mut [T]) };
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(WnfQueryError::InvalidBitPattern)
        }
    }

    unsafe fn query_internal<T>(
        &self,
        buffer: *mut T,
        buffer_size: usize,
    ) -> Result<(usize, WnfChangeStamp), windows::core::Error> {
        let mut change_stamp = WnfChangeStamp::default();
        let mut size = buffer_size as u32;

        let result = ntdll_sys::ZwQueryWnfStateData(
            &self.state_name.opaque_value(),
            ptr::null(),
            ptr::null(),
            change_stamp.as_mut_ptr(),
            buffer.cast(),
            &mut size,
        );

        if result.is_err() && (result != STATUS_BUFFER_TOO_SMALL || size as usize <= buffer_size) {
            Err(result.into())
        } else {
            Ok((size as usize, change_stamp))
        }
    }

    pub fn set<T>(&self, data: &T) -> Result<(), WnfUpdateError>
    where
        T: NoUninit,
    {
        self.update(data, None)?;
        Ok(())
    }

    pub fn set_slice<T>(&self, data: &[T]) -> Result<(), WnfUpdateError>
    where
        T: NoUninit,
    {
        self.update_slice(data, None)?;
        Ok(())
    }

    pub fn update<T>(&self, data: &T, expected_change_stamp: Option<WnfChangeStamp>) -> Result<bool, WnfUpdateError>
    where
        T: NoUninit,
    {
        self.update_slice(slice::from_ref(data), expected_change_stamp)
    }

    pub fn update_slice<T>(
        &self,
        data: &[T],
        expected_change_stamp: Option<WnfChangeStamp>,
    ) -> Result<bool, WnfUpdateError>
    where
        T: NoUninit,
    {
        let result = unsafe {
            ntdll_sys::ZwUpdateWnfStateData(
                &self.state_name.opaque_value(),
                data.as_ptr().cast(),
                (data.len() * mem::size_of::<T>()) as u32,
                ptr::null(),
                ptr::null(),
                expected_change_stamp.unwrap_or_default().into(),
                expected_change_stamp.is_some() as u32,
            )
        };

        if expected_change_stamp.is_some() && result == STATUS_WAIT_1 {
            Ok(false)
        } else {
            result.ok()?;
            Ok(true)
        }
    }

    pub fn apply<T>(&self, mut op: impl FnMut(&T) -> T) -> Result<(), WnfApplyError>
    where
        T: CheckedBitPattern + NoUninit,
    {
        loop {
            let query_result = self.query()?;
            if self.update(&op(query_result.data()), Some(query_result.change_stamp()))? {
                break;
            }
        }

        Ok(())
    }

    pub fn apply_slice<T>(&self, mut op: impl FnMut(&[T]) -> Box<[T]>) -> Result<(), WnfApplyError>
    where
        T: CheckedBitPattern + NoUninit,
    {
        loop {
            let query_result = self.query_slice()?;
            if self.update_slice(&op(query_result.data()), Some(query_result.change_stamp()))? {
                break;
            }
        }

        Ok(())
    }

    pub fn subscribe<'a, T, F>(&self, listener: Box<F>) -> Result<WnfSubscriptionHandle<'a, F>, WnfSubscribeError>
    where
        T: CheckedBitPattern,
        F: FnMut(Option<WnfStampedData<&T>>) + Send + ?Sized + 'static,
    {
        self.subscribe_internal(listener)
    }

    pub fn subscribe_slice<'a, T, F>(&self, listener: Box<F>) -> Result<WnfSubscriptionHandle<'a, F>, WnfSubscribeError>
    where
        T: CheckedBitPattern,
        F: FnMut(Option<WnfStampedData<&[T]>>) + Send + ?Sized + 'static,
    {
        self.subscribe_internal(listener)
    }

    fn subscribe_internal<'a, D, F>(&self, listener: Box<F>) -> Result<WnfSubscriptionHandle<'a, F>, WnfSubscribeError>
    where
        D: FromBuffer + ?Sized,
        F: FnMut(Option<WnfStampedData<&D>>) + Send + ?Sized + 'static,
    {
        extern "system" fn callback<
            D: FromBuffer + ?Sized,
            F: FnMut(Option<WnfStampedData<&D>>) + Send + ?Sized + 'static,
        >(
            _state_name: u64,
            change_stamp: u32,
            _type_id: *const GUID,
            context: *mut c_void,
            buffer: *const c_void,
            buffer_size: u32,
        ) -> NTSTATUS {
            let _ = panic::catch_unwind(|| {
                let context: &WnfSubscriptionContext<F> = unsafe { &*context.cast() };

                context.with_listener(|listener| {
                    let maybe_data = unsafe { D::from_buffer(buffer, buffer_size) };
                    let stamped_data =
                        maybe_data.map(|data| WnfStampedData::from_data_change_stamp(data, change_stamp));
                    (*listener)(stamped_data);
                });
            });

            STATUS_SUCCESS
        }

        let mut subscription = 0;
        let context = Box::new(WnfSubscriptionContext::new(listener));

        unsafe {
            ntdll_sys::RtlSubscribeWnfStateChangeNotification(
                &mut subscription,
                self.state_name.opaque_value(),
                0,
                callback::<D, F>,
                &*context as *const _ as *mut c_void,
                ptr::null(),
                0,
                0,
            )
        }
        .ok()?;

        Ok(WnfSubscriptionHandle::new(context, subscription))
    }
}

trait FromBuffer {
    unsafe fn from_buffer<'a>(buffer: *const c_void, buffer_size: u32) -> Option<&'a Self>;
}

impl<T> FromBuffer for T
where
    T: CheckedBitPattern,
{
    unsafe fn from_buffer<'a>(buffer: *const c_void, buffer_size: u32) -> Option<&'a Self> {
        if buffer as usize % mem::align_of::<T>() != 0 || buffer_size as usize != mem::size_of::<T>() {
            return None;
        }

        let bits: &T::Bits = &*buffer.cast();

        if T::is_valid_bit_pattern(bits) {
            Some(&*buffer.cast())
        } else {
            None
        }
    }
}

impl<T> FromBuffer for [T]
where
    T: CheckedBitPattern,
{
    unsafe fn from_buffer<'a>(buffer: *const c_void, buffer_size: u32) -> Option<&'a Self> {
        if buffer as usize % mem::align_of::<T>() != 0 {
            return None;
        }

        if mem::size_of::<T>() == 0 {
            return Some(&[]);
        }

        if buffer_size as usize % mem::size_of::<T>() != 0 {
            return None;
        }

        let data = slice::from_raw_parts(buffer.cast(), buffer_size as usize / mem::size_of::<T>());

        if data.iter().all(T::is_valid_bit_pattern) {
            let data = &*(data as *const [T::Bits] as *const [T]);
            Some(data)
        } else {
            None
        }
    }
}
