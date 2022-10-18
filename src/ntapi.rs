//! Raw bindings to some of the WNF functions of the Windows Native API
//!
//! *Note*: This is an undocumented part of the Windows API. The information given in the function documentations in
//! this module has been collected from various sources and via reverse engineering. There is no guarantee that it is
//! correct. This applies in particular to the safety conditions.
//!
//! Functions whose names start with `Rtl` (standing for "runtime library") provide higher-level abstractions while
//! functions whose names start with `Nt` are more low level. We use a combination of both, choosing whichever function
//! is more suitable for the task at hand.

#![deny(unsafe_code)]

use std::ffi::c_void;

use windows::{core::GUID, Win32::Foundation::NTSTATUS};

pub(crate) const TRACING_TARGET: &str = "wnf::ntapi";

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

    pub(crate) fn NtCreateWnfStateName(
        state_name: *mut u64,
        name_lifetime: u32,
        data_scope: u32,
        persist_data: u8,
        type_id: *const GUID,
        maximum_state_size: u32,
        security_descriptor: *mut c_void,
    ) -> NTSTATUS;

    pub(crate) fn NtDeleteWnfStateName(state_name: *const u64) -> NTSTATUS;

    pub(crate) fn NtQueryWnfStateData(
        state_name: *const u64,
        type_id: *const GUID,
        explicit_scope: *const c_void,
        change_stamp: *mut u32,
        buffer: *mut c_void,
        buffer_size: *mut u32,
    ) -> NTSTATUS;

    /// Queries information about a WNF state name
    ///
    /// The information is written into a 4-byte buffer and is always a boolean value, where `0` means `false` and `1`
    /// means `true`.
    ///
    /// # Arguments
    /// - `state_name`: Pointer to the WNF state name
    /// - `name_info_class`: Tag of the class of information to obtain
    ///   At least the following values are valid:
    ///   - `0`: "State name exist"
    ///   - `1`: "Subscribers present"
    ///   - `2`: "Is quiescent"
    /// - `explicit_scope`: Irrelevant, can be a null pointer
    /// - `buffer`: Pointer to a buffer the information will be written to, usually having the layout of a `u32`
    /// - `buffer_size`: Size of the buffer in bytes, usually `4`
    ///
    /// # Returns
    /// An `NTSTATUS` value that is `>= 0` on success and `< 0` on failure.
    ///
    /// # Safety
    /// - `state_name` must point to a valid `u64`
    /// - `buffer` must be valid for writes of `u32`
    /// - `buffer_size` must be `4`
    pub(crate) fn NtQueryWnfStateNameInformation(
        state_name: *const u64,
        name_info_class: u32,
        explicit_scope: *const c_void,
        buffer: *mut c_void,
        buffer_size: u32,
    ) -> NTSTATUS;

    /// Updates the data of a WNF state
    ///
    /// # Arguments
    /// - `state_name`: Pointer to the WNF state name
    /// - `buffer`: Pointer to a buffer the data will be read from
    /// - `buffer_size`: Size of the buffer in bytes
    /// - `type_id`: Pointer to a GUID used as the type ID, can be a null pointer
    /// - `explicit_scope`: Irrelevant, can be a null pointer
    /// - `matching_change_stamp`: The expected current change stamp of the state
    ///   (only relevant if `check_stamp` is `1`)
    /// - `check_stamp`:
    ///   `1` if the update should only be performed if the current change stamp equals `matching_change_stamp`,
    ///   `0` if the update should be performed regardless of the current change stamp
    ///
    /// # Returns
    /// An `NTSTATUS` value that is `>= 0` on success and `< 0` on failure.
    ///
    /// In particular, returns [`STATUS_UNSUCCESSFUL`](windows::Win32::Foundation::STATUS_UNSUCCESSFUL) if the update
    /// was not performed because `check_stamp` was `1` and the current change stamp was different from
    /// `matching_change_stamp`.
    ///
    /// # Safety
    /// - `state_name` must point to a valid `u64`
    /// - `buffer` must be valid for reads of at least size `buffer_size`
    /// - The memory range of size `buffer_size` starting at `buffer` must be initialized
    /// - `type_id` must either be a null pointer or point to a valid [`GUID`]
    pub(crate) fn NtUpdateWnfStateData(
        state_name: *const u64,
        buffer: *const c_void,
        buffer_size: u32,
        type_id: *const GUID,
        explicit_scope: *const c_void,
        matching_change_stamp: u32,
        check_stamp: u32,
    ) -> NTSTATUS;
}
