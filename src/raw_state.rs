use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::{mem, panic, ptr, slice};

use windows::core::GUID;
use windows::Win32::Foundation::{
    NTSTATUS, STATUS_BUFFER_TOO_SMALL, STATUS_OBJECT_NAME_NOT_FOUND, STATUS_SUCCESS, STATUS_WAIT_1,
};

use crate::data::WnfStateInfo;
use crate::error::{WnfApplyError, WnfDeleteError, WnfInfoError, WnfQueryError, WnfSubscribeError, WnfUpdateError};
use crate::subscription::{WnfSubscriptionContext, WnfSubscriptionHandle};
use crate::{
    ntdll_sys, Pod, SecurityDescriptor, WnfChangeStamp, WnfCreateError, WnfDataScope, WnfStampedData, WnfStateName,
    WnfStateNameLifetime,
};

// conceptually: *mut State<T>
#[derive(Debug)]
pub(crate) struct RawWnfState<T> {
    state_name: WnfStateName,
    _marker: PhantomData<fn(T) -> T>,
}

impl<T> Copy for RawWnfState<T> {}

impl<T> Clone for RawWnfState<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> RawWnfState<T> {
    pub(crate) fn from_state_name(state_name: WnfStateName) -> Self {
        Self {
            state_name,
            _marker: PhantomData,
        }
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

impl<T: Pod> RawWnfState<T> {
    pub fn get(&self) -> Result<T, WnfQueryError> {
        self.query().map(WnfStampedData::into_data)
    }

    pub fn get_slice(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.query_slice().map(WnfStampedData::into_data)
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        let mut buffer = MaybeUninit::<T>::uninit();
        let (size, change_stamp) = unsafe { self.query_internal(buffer.as_mut_ptr(), mem::size_of::<T>())? };

        if size == mem::size_of::<T>() {
            let data = unsafe { buffer.assume_init() };
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(WnfQueryError::WrongSize {
                expected: mem::size_of::<T>(),
                actual: size,
            })
        }
    }

    pub fn query_slice(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError> {
        let mut buffer = Vec::new();

        let (len, change_stamp) = loop {
            let (size, change_stamp) =
                unsafe { self.query_internal(buffer.as_mut_ptr(), buffer.capacity() * mem::size_of::<T>())? };

            if size == 0 {
                break (0, change_stamp);
            }

            if mem::size_of::<T>() == 0 {
                return Err(WnfQueryError::WrongSize {
                    expected: 0,
                    actual: size,
                });
            }

            if size % mem::size_of::<T>() != 0 {
                return Err(WnfQueryError::WrongSizeMultiple {
                    expected_modulus: mem::size_of::<T>(),
                    actual: size,
                });
            }

            let len = size / mem::size_of::<T>();
            if len > buffer.capacity() {
                buffer.reserve(len - buffer.capacity());
            } else {
                break (len, change_stamp);
            }
        };

        unsafe {
            buffer.set_len(len);
        }

        Ok(WnfStampedData::from_data_change_stamp(
            buffer.into_boxed_slice(),
            change_stamp,
        ))
    }

    unsafe fn query_internal(
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

    pub fn set(&self, data: &T) -> Result<(), WnfUpdateError> {
        self.update(data, None)?;
        Ok(())
    }

    pub fn set_slice(&self, data: &[T]) -> Result<(), WnfUpdateError> {
        self.update_slice(data, None)?;
        Ok(())
    }

    pub fn update(&self, data: &T, expected_change_stamp: Option<WnfChangeStamp>) -> Result<bool, WnfUpdateError> {
        self.update_slice(slice::from_ref(data), expected_change_stamp)
    }

    pub fn update_slice(
        &self,
        data: &[T],
        expected_change_stamp: Option<WnfChangeStamp>,
    ) -> Result<bool, WnfUpdateError> {
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

    pub fn apply(&self, mut op: impl FnMut(&T) -> T) -> Result<(), WnfApplyError> {
        loop {
            let query_result = self.query()?;
            if self.update(&op(query_result.data()), Some(query_result.change_stamp()))? {
                break;
            }
        }

        Ok(())
    }

    pub fn apply_slice(&self, mut op: impl FnMut(&[T]) -> Box<[T]>) -> Result<(), WnfApplyError> {
        loop {
            let query_result = self.query_slice()?;
            if self.update_slice(&op(query_result.data()), Some(query_result.change_stamp()))? {
                break;
            }
        }

        Ok(())
    }

    pub fn subscribe<'a, F: FnMut(Option<WnfStampedData<&T>>) + Send + ?Sized + 'static>(
        &self,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<'a, F>, WnfSubscribeError> {
        self.subscribe_internal(listener)
    }

    pub fn subscribe_slice<'a, F: FnMut(Option<WnfStampedData<&[T]>>) + Send + ?Sized + 'static>(
        &self,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<'a, F>, WnfSubscribeError> {
        self.subscribe_internal(listener)
    }

    fn subscribe_internal<
        'a,
        D: FromBuffer + ?Sized,
        F: FnMut(Option<WnfStampedData<&D>>) + Send + ?Sized + 'static,
    >(
        &self,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<'a, F>, WnfSubscribeError> {
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

impl<T: Pod> FromBuffer for T {
    unsafe fn from_buffer<'a>(buffer: *const c_void, buffer_size: u32) -> Option<&'a Self> {
        if buffer as usize % mem::align_of::<T>() == 0 && buffer_size as usize == mem::size_of::<T>() {
            Some(&*buffer.cast())
        } else {
            None
        }
    }
}

impl<T: Pod> FromBuffer for [T] {
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

        Some(slice::from_raw_parts(
            buffer.cast(),
            buffer_size as usize / mem::size_of::<T>(),
        ))
    }
}
