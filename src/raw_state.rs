use std::alloc::Layout;
use std::borrow::Borrow;
use std::ffi::c_void;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::{alloc, fmt, mem, panic, ptr, slice};

use tracing::{debug, trace_span};
use windows::core::GUID;
use windows::Win32::Foundation::{NTSTATUS, STATUS_BUFFER_TOO_SMALL, STATUS_SUCCESS, STATUS_UNSUCCESSFUL};

use crate::bytes::{CheckedBitPattern, NoUninit};
use crate::callback::WnfCallback;
use crate::data::WnfNameInfoClass;
use crate::error::{
    WnfApplyError, WnfDeleteError, WnfInfoError, WnfQueryError, WnfSubscribeError, WnfTransformError, WnfUpdateError,
};
use crate::ntdll::NTDLL_TARGET;
use crate::subscription::{WnfSubscriptionContext, WnfSubscriptionHandle};
use crate::{
    ntdll_sys, SecurityDescriptor, WnfChangeStamp, WnfCreateError, WnfDataScope, WnfStampedData, WnfStateName,
    WnfStateNameLifetime,
};

pub(crate) struct RawWnfState<T> {
    state_name: WnfStateName,
    _marker: PhantomData<fn(T) -> T>,
}

// cannot derive these as that would impose unnecessary trait bounds on T
impl<T> Copy for RawWnfState<T> {}

impl<T> Clone for RawWnfState<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq<Self> for RawWnfState<T> {
    fn eq(&self, other: &Self) -> bool {
        self.state_name == other.state_name
    }
}

impl<T> Eq for RawWnfState<T> {}

impl<T> Hash for RawWnfState<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.state_name.hash(state);
    }
}

impl<T> Debug for RawWnfState<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawWnfState")
            .field("state_name", &self.state_name)
            .finish()
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

    pub(crate) fn cast<U>(self) -> RawWnfState<U> {
        RawWnfState::from_state_name(self.state_name)
    }

    pub(crate) fn create_temporary() -> Result<Self, WnfCreateError> {
        let mut opaque_value = 0;

        // TODO Can we drop this or is it "borrowed" by the created WNF state?
        let security_descriptor = SecurityDescriptor::create_everyone_generic_all()?;

        let name_lifetime = WnfStateNameLifetime::Temporary as u32;
        let data_scope = WnfDataScope::Machine as u32;
        let persist_data = 0;
        let maximum_state_size = 0x1000;

        let result = unsafe {
            ntdll_sys::ZwCreateWnfStateName(
                &mut opaque_value,
                name_lifetime,
                data_scope,
                persist_data,
                ptr::null(),
                maximum_state_size,
                security_descriptor.as_void_ptr(),
            )
        };

        if result.is_ok() {
            let state_name = WnfStateName::from_opaque_value(opaque_value);

            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.name_lifetime = name_lifetime,
                input.data_scope = data_scope,
                input.persist_data = persist_data,
                input.maximum_state_size = maximum_state_size,
                output.state_name = %state_name,
                "ZwCreateWnfStateName",
            );

            Ok(Self::from_state_name(state_name))
        } else {
            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.name_lifetime = name_lifetime,
                input.data_scope = data_scope,
                input.persist_data = persist_data,
                input.maximum_state_size = maximum_state_size,
                "ZwCreateWnfStateName",
            );

            Err(result.into())
        }
    }

    pub(crate) fn delete(self) -> Result<(), WnfDeleteError> {
        let result = unsafe { ntdll_sys::ZwDeleteWnfStateName(&self.state_name.opaque_value()) };

        debug!(
            target: NTDLL_TARGET,
            ?result,
            input.state_name = %self.state_name,
            "ZwDeleteWnfStateName",
        );

        result.ok()?;
        Ok(())
    }

    pub fn exists(&self) -> Result<bool, WnfInfoError> {
        self.info_internal(WnfNameInfoClass::StateNameExist)
    }

    pub fn subscribers_present(&self) -> Result<bool, WnfInfoError> {
        self.info_internal(WnfNameInfoClass::SubscribersPresent)
    }

    pub fn is_quiescent(&self) -> Result<bool, WnfInfoError> {
        self.info_internal(WnfNameInfoClass::IsQuiescent)
    }

    fn info_internal(&self, name_info_class: WnfNameInfoClass) -> Result<bool, WnfInfoError> {
        let mut buffer = u32::MAX;
        let name_info_class = name_info_class as u32;

        let result = unsafe {
            ntdll_sys::ZwQueryWnfStateNameInformation(
                &self.state_name.opaque_value(),
                name_info_class,
                ptr::null(),
                &mut buffer as *mut _ as *mut c_void,
                mem::size_of_val(&buffer) as u32,
            )
        };

        if result.is_ok() {
            debug!(
                 target: NTDLL_TARGET,
                 ?result,
                 input.state_name = %self.state_name,
                 input.name_info_class = name_info_class,
                 output.buffer = buffer,
                 "ZwQueryWnfStateNameInformation",
            );

            Ok(match buffer {
                0 => false,
                1 => true,
                _ => unreachable!("ZwQueryWnfStateNameInformation did not produce valid boolean"),
            })
        } else {
            debug!(
                 target: NTDLL_TARGET,
                 ?result,
                 input.state_name = %self.state_name,
                 input.name_info_class = name_info_class,
                 "ZwQueryWnfStateNameInformation",
            );

            Err(result.into())
        }
    }
}

impl<T> RawWnfState<T>
where
    T: CheckedBitPattern,
{
    pub fn get(&self) -> Result<T, WnfQueryError> {
        self.query().map(WnfStampedData::into_data)
    }

    pub fn get_boxed(&self) -> Result<Box<T>, WnfQueryError> {
        self.query_boxed().map(WnfStampedData::into_data)
    }

    pub fn get_slice(&self) -> Result<Box<[T]>, WnfQueryError> {
        self.query_slice().map(WnfStampedData::into_data)
    }

    pub fn query(&self) -> Result<WnfStampedData<T>, WnfQueryError> {
        let mut bits = MaybeUninit::<T::Bits>::uninit();

        let (size, change_stamp) = unsafe { self.query_internal(bits.as_mut_ptr(), mem::size_of::<T::Bits>()) }?;
        if size != mem::size_of::<T::Bits>() {
            return Err(WnfQueryError::WrongSize {
                expected: mem::size_of::<T::Bits>(),
                actual: size,
            });
        }

        let bits = unsafe { bits.assume_init() };

        if T::is_valid_bit_pattern(&bits) {
            let data = unsafe { *(&bits as *const T::Bits as *const T) };
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(WnfQueryError::InvalidBitPattern)
        }
    }

    pub fn query_boxed(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError> {
        let mut bits = if mem::size_of::<T::Bits>() == 0 {
            let data = unsafe { mem::zeroed() };
            Box::new(data)
        } else {
            let layout = Layout::new::<T::Bits>();
            let data = unsafe { alloc::alloc(layout) } as *mut MaybeUninit<T::Bits>;
            unsafe { Box::from_raw(data) }
        };

        let (size, change_stamp) = unsafe { self.query_internal(bits.as_mut_ptr(), mem::size_of::<T::Bits>()) }?;
        if size != mem::size_of::<T::Bits>() {
            return Err(WnfQueryError::WrongSize {
                expected: mem::size_of::<T::Bits>(),
                actual: size,
            });
        }

        let bits = unsafe { Box::from_raw(Box::into_raw(bits) as *mut T::Bits) };

        if T::is_valid_bit_pattern(&bits) {
            let data = unsafe { Box::from_raw(Box::into_raw(bits) as *mut T) };
            Ok(WnfStampedData::from_data_change_stamp(data, change_stamp))
        } else {
            Err(WnfQueryError::InvalidBitPattern)
        }
    }

    pub fn query_slice(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError> {
        let stride = Layout::new::<T::Bits>().pad_to_align().size();
        let mut buffer: Vec<T::Bits> = Vec::new();

        let (len, change_stamp) = loop {
            let (size, change_stamp) = unsafe { self.query_internal(buffer.as_mut_ptr(), buffer.capacity() * stride) }?;

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

            let len = size / stride;
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

    unsafe fn query_internal(
        &self,
        buffer: *mut T::Bits,
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
            debug!(
                 target: NTDLL_TARGET,
                 ?result,
                 input.state_name = %self.state_name,
                 "ZwQueryWnfStateData",
            );

            Err(result.into())
        } else {
            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.state_name = %self.state_name,
                output.change_stamp = %change_stamp,
                output.buffer_size = size,
                "ZwQueryWnfStateData",
            );

            Ok((size as usize, change_stamp))
        }
    }
}

impl<T> RawWnfState<T>
where
    T: NoUninit,
{
    pub fn set<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.set_slice(slice::from_ref(data.borrow()))
    }

    pub fn set_slice<D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        D: Borrow<[T]>,
    {
        Ok(self.update_slice_internal(data, None).ok()?)
    }

    pub fn update<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<T>,
    {
        self.update_slice(slice::from_ref(data.borrow()), expected_change_stamp)
    }

    pub fn update_slice<D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        D: Borrow<[T]>,
    {
        let result = self.update_slice_internal(data, Some(expected_change_stamp));

        Ok(if result == STATUS_UNSUCCESSFUL {
            false
        } else {
            result.ok()?;
            true
        })
    }

    pub fn update_slice_internal<D>(&self, data: D, expected_change_stamp: Option<WnfChangeStamp>) -> NTSTATUS
    where
        D: Borrow<[T]>,
    {
        let data = data.borrow();
        let buffer_size = (data.len() * mem::size_of::<T>()) as u32; // T: NoUninit should imply that this is the correct size
        let matching_change_stamp = expected_change_stamp.unwrap_or_default().into();
        let check_stamp = expected_change_stamp.is_some() as u32;

        let result = unsafe {
            ntdll_sys::ZwUpdateWnfStateData(
                &self.state_name.opaque_value(),
                data.as_ptr().cast(),
                buffer_size,
                ptr::null(),
                ptr::null(),
                matching_change_stamp,
                check_stamp,
            )
        };

        debug!(
            target: NTDLL_TARGET,
            ?result,
            input.state_name = %self.state_name,
            input.buffer_size = buffer_size,
            input.matching_change_stamp = matching_change_stamp,
            input.check_stamp = check_stamp,
            "ZwUpdateWnfStateData",
        );

        result
    }
}

impl<T> RawWnfState<T>
where
    T: CheckedBitPattern + NoUninit,
{
    pub fn apply<D, F>(&self, mut transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(T) -> Option<D>,
    {
        loop {
            let (data, change_stamp) = self.query()?.into_data_change_stamp();
            match transform(data) {
                None => return Ok(false),
                Some(data) => {
                    if self.update(data, change_stamp)? {
                        return Ok(true);
                    }
                }
            }
        }
    }

    pub fn apply_boxed<D, F>(&self, mut transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Option<D>,
    {
        loop {
            let (data, change_stamp) = self.query_boxed()?.into_data_change_stamp();
            match transform(data) {
                None => return Ok(false),
                Some(data) => {
                    if self.update(data, change_stamp)? {
                        return Ok(true);
                    }
                }
            }
        }
    }

    pub fn apply_slice<D, F>(&self, mut transform: F) -> Result<bool, WnfApplyError>
    where
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Option<D>,
    {
        loop {
            let (data, change_stamp) = self.query_slice()?.into_data_change_stamp();
            match transform(data) {
                None => return Ok(false),
                Some(data) => {
                    if self.update_slice(data, change_stamp)? {
                        return Ok(true);
                    }
                }
            }
        }
    }

    pub fn try_apply<D, E, F>(&self, mut transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(T) -> Result<Option<D>, E>,
    {
        loop {
            let (data, change_stamp) = self.query()?.into_data_change_stamp();
            match transform(data).map_err(WnfTransformError::from)? {
                None => return Ok(false),
                Some(data) => {
                    if self.update(data, change_stamp)? {
                        return Ok(true);
                    }
                }
            }
        }
    }

    pub fn try_apply_boxed<D, E, F>(&self, mut transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<T>,
        F: FnMut(Box<T>) -> Result<Option<D>, E>,
    {
        loop {
            let (data, change_stamp) = self.query_boxed()?.into_data_change_stamp();
            match transform(data).map_err(WnfTransformError::from)? {
                None => return Ok(false),
                Some(data) => {
                    if self.update(data, change_stamp)? {
                        return Ok(true);
                    }
                }
            }
        }
    }

    pub fn try_apply_slice<D, E, F>(&self, mut transform: F) -> Result<bool, WnfApplyError<E>>
    where
        D: Borrow<[T]>,
        F: FnMut(Box<[T]>) -> Result<Option<D>, E>,
    {
        loop {
            let (data, change_stamp) = self.query_slice()?.into_data_change_stamp();
            match transform(data).map_err(WnfTransformError::from)? {
                None => return Ok(false),
                Some(data) => {
                    if self.update_slice(data, change_stamp)? {
                        return Ok(true);
                    }
                }
            }
        }
    }
}

impl<T> RawWnfState<T>
where
    T: CheckedBitPattern,
{
    pub fn subscribe<F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: WnfCallback<T, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        self.subscribe_internal::<Value<T>, F, ArgsValid, ArgsInvalid>(after_change_stamp, listener)
    }

    pub fn subscribe_boxed<F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: WnfCallback<Box<T>, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        self.subscribe_internal::<Boxed<T>, F, ArgsValid, ArgsInvalid>(after_change_stamp, listener)
    }

    pub fn subscribe_slice<F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        F: WnfCallback<Box<[T]>, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        self.subscribe_internal::<BoxedSlice<T>, F, ArgsValid, ArgsInvalid>(after_change_stamp, listener)
    }

    fn subscribe_internal<B, F, ArgsValid, ArgsInvalid>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        B: FromByteBuffer,
        F: WnfCallback<B::Data, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
    {
        extern "system" fn callback<B, F, ArgsValid, ArgsInvalid>(
            state_name: u64,
            change_stamp: u32,
            _type_id: *const GUID,
            context: *mut c_void,
            buffer: *const c_void,
            buffer_size: u32,
        ) -> NTSTATUS
        where
            B: FromByteBuffer,
            F: WnfCallback<B::Data, ArgsValid, ArgsInvalid> + Send + ?Sized + 'static,
        {
            let _ = panic::catch_unwind(|| {
                let span = trace_span!(
                    target: NTDLL_TARGET,
                    "WnfUserCallback",
                    input.state_name = %WnfStateName::from_opaque_value(state_name),
                    input.change_stamp = change_stamp,
                    input.buffer_size = buffer_size
                );
                let _enter = span.enter();

                let context: &WnfSubscriptionContext<F> = unsafe { &*context.cast() };
                let maybe_data = unsafe { B::from_byte_buffer(buffer, buffer_size) };

                context.with_listener(|listener| match maybe_data {
                    Some(data) => {
                        listener.call_valid(data, change_stamp.into());
                    }
                    None => {
                        listener.call_invalid(change_stamp.into());
                    }
                });
            });

            STATUS_SUCCESS
        }

        let mut subscription = 0;
        let context = Box::new(WnfSubscriptionContext::new(listener));

        let result = unsafe {
            ntdll_sys::RtlSubscribeWnfStateChangeNotification(
                &mut subscription,
                self.state_name.opaque_value(),
                after_change_stamp.into(),
                callback::<B, F, ArgsValid, ArgsInvalid>,
                &*context as *const _ as *mut c_void,
                ptr::null(),
                0,
                0,
            )
        };

        if result.is_ok() {
            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.state_name = %self.state_name,
                input.after_change_stamp = %after_change_stamp,
                output.subscription = subscription,
                "RtlSubscribeWnfStateChangeNotification",
            );

            Ok(WnfSubscriptionHandle::new(context, subscription))
        } else {
            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.state_name = %self.state_name,
                input.after_change_stamp = %after_change_stamp,
                "RtlSubscribeWnfStateChangeNotification",
            );

            Err(result.into())
        }
    }
}

trait FromByteBuffer {
    type Data;

    unsafe fn from_byte_buffer(ptr: *const c_void, size: u32) -> Option<Self::Data>;
}

#[derive(Debug)]
struct Value<T>(PhantomData<fn() -> T>);

impl<T> FromByteBuffer for Value<T>
where
    T: CheckedBitPattern,
{
    type Data = T;

    unsafe fn from_byte_buffer(ptr: *const c_void, size: u32) -> Option<T> {
        if size as usize != mem::size_of::<T::Bits>() {
            return None;
        }

        let bits: T::Bits = ptr::read_unaligned(ptr.cast());

        if T::is_valid_bit_pattern(&bits) {
            Some(*(&bits as *const T::Bits as *const T))
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct Boxed<T>(PhantomData<fn() -> Box<T>>);

impl<T> FromByteBuffer for Boxed<T>
where
    T: CheckedBitPattern,
{
    type Data = Box<T>;

    unsafe fn from_byte_buffer(ptr: *const c_void, size: u32) -> Option<Box<T>> {
        if size as usize != mem::size_of::<T::Bits>() {
            return None;
        }

        let bits = if mem::size_of::<T::Bits>() == 0 {
            Box::new(mem::zeroed())
        } else {
            let layout = Layout::new::<T::Bits>();
            let data = alloc::alloc(layout) as *mut T::Bits;
            ptr::copy_nonoverlapping(ptr as *const T::Bits, data as *mut T::Bits, 1);
            Box::from_raw(data)
        };

        T::is_valid_bit_pattern(&bits).then(|| Box::from_raw(Box::into_raw(bits) as *mut T))
    }
}

#[derive(Debug)]
struct BoxedSlice<T>(PhantomData<fn() -> Box<[T]>>);

impl<T> FromByteBuffer for BoxedSlice<T>
where
    T: CheckedBitPattern,
{
    type Data = Box<[T]>;

    unsafe fn from_byte_buffer(ptr: *const c_void, size: u32) -> Option<Box<[T]>> {
        if mem::size_of::<T>() == 0 {
            return (size == 0).then(|| Vec::new().into_boxed_slice());
        }

        if size as usize % mem::size_of::<T>() != 0 {
            return None;
        }

        let len = size as usize / mem::size_of::<T>();
        let mut data = Vec::with_capacity(len);
        ptr::copy_nonoverlapping(ptr.cast(), data.as_mut_ptr(), len);
        data.set_len(len);

        if data.iter().all(T::is_valid_bit_pattern) {
            let mut data = ManuallyDrop::new(data);
            let data = Vec::from_raw_parts(data.as_mut_ptr() as *mut T, data.len(), data.capacity());
            Some(data.into_boxed_slice())
        } else {
            None
        }
    }
}
