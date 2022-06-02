use std::alloc::Layout;
use std::borrow::Borrow;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::{alloc, mem, panic, ptr, slice};

use windows::core::GUID;
use windows::Win32::Foundation::{NTSTATUS, STATUS_BUFFER_TOO_SMALL, STATUS_SUCCESS, STATUS_UNSUCCESSFUL};

use crate::bytes::{CheckedBitPattern, NoUninit};
use crate::data::WnfNameInfoClass;
use crate::error::{
    WnfApplyError, WnfDeleteError, WnfInfoError, WnfQueryError, WnfSubscribeError, WnfTransformError, WnfUpdateError,
};
use crate::subscription::{WnfStateChangeListener, WnfSubscriptionContext, WnfSubscriptionHandle};
use crate::{
    ntdll_sys, SecurityDescriptor, WnfChangeStamp, WnfCreateError, WnfDataScope, WnfStampedData, WnfStateName,
    WnfStateNameLifetime,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
        unsafe { ntdll_sys::ZwDeleteWnfStateName(&self.state_name.opaque_value()) }.ok()?;
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

        unsafe {
            ntdll_sys::ZwQueryWnfStateNameInformation(
                &self.state_name.opaque_value(),
                name_info_class as u32,
                ptr::null(),
                &mut buffer as *mut _ as *mut c_void,
                mem::size_of_val(&buffer) as u32,
            )
        }
        .ok()?;

        Ok(match buffer {
            0 => false,
            1 => true,
            _ => unreachable!("ZwQueryWnfStateNameInformation did not produce valid boolean"),
        })
    }

    pub fn get<T>(&self) -> Result<T, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.query().map(WnfStampedData::into_data)
    }

    pub fn get_boxed<T>(&self) -> Result<Box<T>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
        self.query_boxed().map(WnfStampedData::into_data)
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

    pub fn query_boxed<T>(&self) -> Result<WnfStampedData<Box<T>>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
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

    pub fn query_slice<T>(&self) -> Result<WnfStampedData<Box<[T]>>, WnfQueryError>
    where
        T: CheckedBitPattern,
    {
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

    pub fn set<T, D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        T: NoUninit,
        D: Borrow<T>,
    {
        self.set_slice(slice::from_ref(data.borrow()))
    }

    pub fn set_slice<T, D>(&self, data: D) -> Result<(), WnfUpdateError>
    where
        T: NoUninit,
        D: Borrow<[T]>,
    {
        Ok(self.update_slice_internal(data, None).ok()?)
    }

    pub fn update<T, D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        T: NoUninit,
        D: Borrow<T>,
    {
        self.update_slice(slice::from_ref(data.borrow()), expected_change_stamp)
    }

    pub fn update_slice<T, D>(&self, data: D, expected_change_stamp: WnfChangeStamp) -> Result<bool, WnfUpdateError>
    where
        T: NoUninit,
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

    pub fn update_slice_internal<T, D>(&self, data: D, expected_change_stamp: Option<WnfChangeStamp>) -> NTSTATUS
    where
        T: NoUninit,
        D: Borrow<[T]>,
    {
        let data = data.borrow();

        unsafe {
            ntdll_sys::ZwUpdateWnfStateData(
                &self.state_name.opaque_value(),
                data.as_ptr().cast(),
                (data.len() * mem::size_of::<T>()) as u32, // T: NoUninit should imply that this is the correct size
                ptr::null(),
                ptr::null(),
                expected_change_stamp.unwrap_or_default().into(),
                expected_change_stamp.is_some() as u32,
            )
        }
    }

    pub fn apply<T, D, F>(&self, mut transform: F) -> Result<bool, WnfApplyError>
    where
        T: CheckedBitPattern + NoUninit,
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

    pub fn apply_boxed<T, D, F>(&self, mut transform: F) -> Result<bool, WnfApplyError>
    where
        T: CheckedBitPattern + NoUninit,
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

    pub fn apply_slice<T, D, F>(&self, mut transform: F) -> Result<bool, WnfApplyError>
    where
        T: CheckedBitPattern + NoUninit,
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

    pub fn try_apply<T, D, E, F>(&self, mut transform: F) -> Result<bool, WnfApplyError<E>>
    where
        T: CheckedBitPattern + NoUninit,
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

    pub fn try_apply_boxed<T, D, E, F>(&self, mut transform: F) -> Result<bool, WnfApplyError<E>>
    where
        T: CheckedBitPattern + NoUninit,
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

    pub fn try_apply_slice<T, D, E, F>(&self, mut transform: F) -> Result<bool, WnfApplyError<E>>
    where
        T: CheckedBitPattern + NoUninit,
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

    pub fn subscribe<T, F, A>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        T: CheckedBitPattern,
        F: WnfStateChangeListener<T, A> + Send + ?Sized + 'static,
    {
        self.subscribe_internal::<Value<T>, F, A>(after_change_stamp, listener)
    }

    pub fn subscribe_boxed<T, F, A>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        T: CheckedBitPattern,
        F: WnfStateChangeListener<Box<T>, A> + Send + ?Sized + 'static,
    {
        self.subscribe_internal::<Boxed<T>, F, A>(after_change_stamp, listener)
    }

    pub fn subscribe_slice<T, F, A>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        T: CheckedBitPattern,
        F: WnfStateChangeListener<Box<[T]>, A> + Send + ?Sized + 'static,
    {
        self.subscribe_internal::<BoxedSlice<T>, F, A>(after_change_stamp, listener)
    }

    fn subscribe_internal<B, F, A>(
        &self,
        after_change_stamp: WnfChangeStamp,
        listener: Box<F>,
    ) -> Result<WnfSubscriptionHandle<F>, WnfSubscribeError>
    where
        B: FromByteBuffer,
        F: WnfStateChangeListener<B::Data, A> + Send + ?Sized + 'static,
    {
        extern "system" fn callback<B, F, A>(
            _state_name: u64,
            change_stamp: u32,
            _type_id: *const GUID,
            context: *mut c_void,
            buffer: *const c_void,
            buffer_size: u32,
        ) -> NTSTATUS
        where
            B: FromByteBuffer,
            F: WnfStateChangeListener<B::Data, A> + Send + ?Sized + 'static,
        {
            let _ = panic::catch_unwind(|| {
                let context: &WnfSubscriptionContext<F> = unsafe { &*context.cast() };
                let maybe_data = unsafe { B::from_byte_buffer(buffer, buffer_size) };

                context.with_listener(|listener| {
                    listener.call(maybe_data, change_stamp.into());
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
                after_change_stamp.into(),
                callback::<B, F, A>,
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
