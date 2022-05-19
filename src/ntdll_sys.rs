use std::ffi::c_void;
use windows::{core::GUID, Win32::Foundation::WIN32_ERROR};

pub const ERROR_BUFFER_TOO_SMALL: WIN32_ERROR = WIN32_ERROR(0xC0000023);

pub(crate) type WnfUserCallback = extern "system" fn(
    state_name: u64,
    change_stamp: u32,
    type_id: *const GUID,
    context: *mut c_void,
    buffer: *const c_void,
    buffer_size: u32,
) -> WIN32_ERROR;

#[link(name = "ntdll")]
extern "system" {
    pub(crate) fn RtlSubscribeWnfStateChangeNotification(
        subscription: *mut *const c_void,
        state_name: u64,
        change_stamp: u32,
        callback: WnfUserCallback,
        callback_context: *mut c_void,
        type_id: *const GUID,
        serialization_group: u32,
        unknown: u32,
    ) -> WIN32_ERROR;

    pub(crate) fn RtlUnsubscribeWnfStateChangeNotification(
        subscription: *const c_void,
    ) -> WIN32_ERROR;

    pub(crate) fn ZwQueryWnfStateData(
        state_name: *const u64,
        type_id: *const GUID,
        explicit_scope: *const c_void,
        change_stamp: *mut u32,
        buffer: *mut c_void,
        buffer_size: *mut u32,
    ) -> WIN32_ERROR;

    pub(crate) fn ZwUpdateWnfStateData(
        state_name: *const u64,
        buffer: *const c_void,
        buffer_size: u32,
        type_id: *const GUID,
        explicit_scope: *const c_void,
        matching_change_stamp: u32,
        check_stamp: u32,
    ) -> WIN32_ERROR;
}
