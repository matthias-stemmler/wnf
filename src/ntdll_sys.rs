use std::ffi::c_void;

use windows::{core::GUID, Win32::Foundation::NTSTATUS};

pub(crate) type WnfUserCallback = extern "system" fn(
    state_name: u64,
    change_stamp: u32,
    type_id: *const GUID,
    context: *mut c_void,
    buffer: *const c_void,
    buffer_size: u32,
) -> NTSTATUS;

#[link(name = "ntdll")]
extern "system" {
    pub(crate) fn RtlSubscribeWnfStateChangeNotification(
        subscription: *mut u64,
        state_name: u64,
        change_stamp: u32,
        callback: WnfUserCallback,
        callback_context: *mut c_void,
        type_id: *const GUID,
        serialization_group: u32,
        unknown: u32,
    ) -> NTSTATUS;

    pub(crate) fn RtlUnsubscribeWnfStateChangeNotification(subscription: u64) -> NTSTATUS;

    pub(crate) fn ZwCreateWnfStateName(
        state_name: *mut u64,
        name_lifetime: u32,
        data_scope: u32,
        persist_data: u8,
        type_id: *const GUID,
        maximum_state_size: u32,
        security_descriptor: *const c_void,
    ) -> NTSTATUS;

    pub(crate) fn ZwDeleteWnfStateName(state_name: *const u64) -> NTSTATUS;

    pub(crate) fn ZwQueryWnfStateData(
        state_name: *const u64,
        type_id: *const GUID,
        explicit_scope: *const c_void,
        change_stamp: *mut u32,
        buffer: *mut c_void,
        buffer_size: *mut u32,
    ) -> NTSTATUS;

    pub(crate) fn ZwUpdateWnfStateData(
        state_name: *const u64,
        buffer: *const c_void,
        buffer_size: u32,
        type_id: *const GUID,
        explicit_scope: *const c_void,
        matching_change_stamp: u32,
        check_stamp: u32,
    ) -> NTSTATUS;
}
