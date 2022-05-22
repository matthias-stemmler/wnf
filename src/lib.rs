#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

use ntdll_sys::{WnfDataScope, WnfStateNameLifetime};
use pod::Pod;
use security::{SecurityCreateError, SecurityDescriptor};

use std::{
    ffi::c_void,
    marker::PhantomData,
    mem::{self, ManuallyDrop, MaybeUninit},
    panic, ptr, slice,
    sync::Mutex,
};
use thiserror::Error;
use windows::{
    core::GUID,
    Win32::Foundation::{NTSTATUS, STATUS_BUFFER_TOO_SMALL, STATUS_SUCCESS, STATUS_WAIT_1},
};

mod ntdll_sys;
mod pod;
mod security;

// TODO allow specifying minimum change_stamp for subscribe
// TODO maybe extract trait similar to FromBuffer also for query?

pub fn create_temporary_wnf_state_name() -> Result<WnfStateName, WnfCreateError> {
    let mut opaque_value = 0;
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

    Ok(WnfStateName::from_opaque_value(opaque_value))
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WnfStateName(u64);

impl WnfStateName {
    pub const fn from_opaque_value(opaque_value: u64) -> Self {
        Self(opaque_value)
    }

    pub const fn opaque_value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WnfChangeStamp(u32);

impl WnfChangeStamp {
    pub const fn from_value(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }

    fn as_mut_ptr(&mut self) -> *mut u32 {
        &mut self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WnfState<T> {
    state_name: WnfStateName,
    _phantom: PhantomData<fn(T) -> T>,
}

impl<T: Pod> WnfState<T> {
    pub fn from_state_name(state_name: WnfStateName) -> Self {
        Self {
            state_name,
            _phantom: PhantomData,
        }
    }

    pub fn get(&self) -> Result<T, WnfQueryError> {
        self.query().map(WnfStampedData::into_data)
    }

    pub fn get_slice(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.query_slice().map(WnfStampedData::into_data)
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        let mut buffer = MaybeUninit::<T>::uninit();
        let (size, change_stamp) =
            unsafe { self.query_internal(buffer.as_mut_ptr(), mem::size_of::<T>())? };

        if size == mem::size_of::<T>() {
            let data = unsafe { buffer.assume_init() };
            Ok(WnfStampedData { data, change_stamp })
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
            let (size, change_stamp) = unsafe {
                self.query_internal(buffer.as_mut_ptr(), buffer.capacity() * mem::size_of::<T>())?
            };

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

        Ok(WnfStampedData {
            data: buffer.into_boxed_slice(),
            change_stamp,
        })
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

    pub fn update(
        &self,
        data: &T,
        expected_change_stamp: Option<WnfChangeStamp>,
    ) -> Result<bool, WnfUpdateError> {
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
                expected_change_stamp.unwrap_or_default().value(),
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
            if self.update(&op(&query_result.data), Some(query_result.change_stamp))? {
                break;
            }
        }

        Ok(())
    }

    pub fn apply_slice(&self, mut op: impl FnMut(&[T]) -> Box<[T]>) -> Result<(), WnfApplyError> {
        loop {
            let query_result = self.query_slice()?;
            if self.update_slice(&op(&query_result.data), Some(query_result.change_stamp))? {
                break;
            }
        }

        Ok(())
    }

    pub fn subscribe<F: FnMut(Option<WnfStampedData<&T>>) + Send + ?Sized + 'static>(
        &self,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError> {
        self.subscribe_internal(listener)
    }

    pub fn subscribe_slice<F: FnMut(Option<WnfStampedData<&[T]>>) + Send + ?Sized + 'static>(
        &self,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError> {
        self.subscribe_internal(listener)
    }

    fn subscribe_internal<
        D: FromBuffer + ?Sized,
        F: FnMut(Option<WnfStampedData<&D>>) + Send + ?Sized + 'static,
    >(
        &self,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError> {
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

                    let stamped_data = maybe_data.map(|data| WnfStampedData {
                        data,
                        change_stamp: WnfChangeStamp::from_value(change_stamp),
                    });

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

        Ok(WnfSubscriptionHandle(Some(WnfSubscriptionHandleInner {
            context: ManuallyDrop::new(context),
            subscription,
        })))
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WnfStampedData<T> {
    data: T,
    change_stamp: WnfChangeStamp,
}

impl<T> WnfStampedData<T> {
    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn into_data(self) -> T {
        self.data
    }

    pub fn change_stamp(&self) -> WnfChangeStamp {
        self.change_stamp
    }
}

#[derive(Debug)]
pub struct WnfSubscriptionHandle<F: ?Sized>(Option<WnfSubscriptionHandleInner<F>>);

#[derive(Debug)]
pub struct WnfSubscriptionHandleInner<F: ?Sized> {
    context: ManuallyDrop<Box<WnfSubscriptionContext<F>>>,
    subscription: u64,
}

#[derive(Debug)]
pub struct WnfSubscriptionContext<F: ?Sized>(Mutex<Option<Box<F>>>);

impl<F: ?Sized> WnfSubscriptionContext<F> {
    fn new(listener: Box<F>) -> Self {
        Self(Mutex::new(Some(listener)))
    }

    fn reset(&self) {
        let mut listener = match self.0.lock() {
            Ok(context) => context,
            Err(err) => err.into_inner(),
        };

        *listener = None;
    }

    fn with_listener(&self, op: impl FnOnce(&mut F)) {
        if let Ok(mut listener) = self.0.lock() {
            if let Some(listener) = listener.as_mut() {
                op(listener);
            }
        }
    }
}

impl<F: ?Sized> WnfSubscriptionHandle<F> {
    pub fn forget(self) {
        mem::forget(self);
    }

    pub fn unsubscribe(mut self) -> Result<(), (WnfUnsubscribeError, Self)> {
        self.try_unsubscribe().map_err(|err| (err, self))
    }

    fn try_unsubscribe(&mut self) -> Result<(), WnfUnsubscribeError> {
        if let Some(inner) = self.0.take() {
            let result =
                unsafe { ntdll_sys::RtlUnsubscribeWnfStateChangeNotification(inner.subscription) };

            if result.is_ok() {
                ManuallyDrop::into_inner(inner.context);
            } else {
                self.0 = Some(inner);
            }
        };

        Ok(())
    }
}

impl<F: ?Sized> Drop for WnfSubscriptionHandle<F> {
    fn drop(&mut self) {
        if self.try_unsubscribe().is_err() {
            if let Some(inner) = self.0.take() {
                inner.context.reset();
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum WnfCreateError {
    #[error("failed to create WNF state name: security error {0}")]
    Security(#[from] SecurityCreateError),

    #[error("failed to create WNF state name: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Error, Debug)]
pub enum WnfQueryError {
    #[error(
        "failed to query WNF state data: data has wrong size (expected {expected}, got {actual})"
    )]
    WrongSize { expected: usize, actual: usize },

    #[error(
        "failed to query WNF state data: data has wrong size (expected multiple of {expected_modulus}, got {actual})"
    )]
    WrongSizeMultiple {
        expected_modulus: usize,
        actual: usize,
    },

    #[error("failed to query WNF state data: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Error, Debug)]
pub enum WnfUpdateError {
    #[error("failed to update WNF state data: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Error, Debug)]
pub enum WnfApplyError {
    #[error("failed to apply operation to WNF state data: failed to query data {0}")]
    Query(#[from] WnfQueryError),

    #[error("failed to apply operation to WNF state data: failed to update data {0}")]
    Update(#[from] WnfUpdateError),
}

#[derive(Error, Debug)]
pub enum WnfSubscribeError {
    #[error("failed to subscribe to WNF state change: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Error, Debug)]
pub enum WnfUnsubscribeError {
    #[error("failed to unsubscribe from WNF state change: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

trait FromBuffer {
    unsafe fn from_buffer<'a>(buffer: *const c_void, buffer_size: u32) -> Option<&'a Self>;
}

impl<T: Pod> FromBuffer for T {
    unsafe fn from_buffer<'a>(buffer: *const c_void, buffer_size: u32) -> Option<&'a Self> {
        if buffer as usize % mem::align_of::<T>() == 0
            && buffer_size as usize == mem::size_of::<T>()
        {
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

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::*;

    #[test]
    fn test() {
        let state_name = create_temporary_wnf_state_name().unwrap();
        println!("{:#010x}", state_name.opaque_value());

        let state: WnfState<u32> = WnfState::from_state_name(state_name);

        let _handle = state
            .subscribe(Box::new(|data: Option<WnfStampedData<&u32>>| {
                println!("{data:?}");
            }))
            .unwrap();

        state.set(&100).unwrap();

        let join_handle = thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(500));
                state.apply(|x| x + 1).unwrap();
            }

        });

        join_handle.join().unwrap();
    }
}
