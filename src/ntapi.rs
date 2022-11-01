//! Raw bindings to some of the WNF functions of the Windows Native API (NTAPI)
//!
//! *Note*: This is an undocumented part of the Windows API. The information given in the function documentations in
//! this module has been collected from various sources and via reverse engineering. There is no guarantee that it is
//! correct. This applies in particular to the safety conditions.
//!
//! Functions whose names start with `Rtl` (standing for "runtime library") provide higher-level abstractions while
//! functions whose names start with `Nt` are more low level. We use a combination of both, choosing whichever function
//! is more suitable for the task at hand.
//!
//! Each of the `Nt` functions has a counterpart whose name starts with `Zw` instead. For calls from user mode (which is
//! the only mode supported by this crate), these two classes of functions behave identically.
//!
//! See also <https://learn.microsoft.com/en-us/windows-hardware/drivers/kernel/using-nt-and-zw-versions-of-the-native-system-services-routines>

#![deny(unsafe_code)]

use std::ffi::c_void;

use windows::core::GUID;
use windows::Win32::Foundation::NTSTATUS;
use windows::Win32::Security::PSECURITY_DESCRIPTOR;

/// Target used for logging calls to NTAPI functions using the `tracing` crate
pub(crate) const TRACING_TARGET: &str = "wnf::ntapi";

/// A callback function for a state subscription
///
/// # Arguments
/// - [in] `state_name`: The state name
/// - [in] `change_stamp`: The current change stamp of the state
/// - [in] `type_id`: Pointer to a GUID used as the type ID, may be a null pointer
/// - [in] `context`: Opaque pointer to arbitrary context data passed to `RtlSubscribeWnfStateChangeNotification`
/// - [in] `buffer`: Pointer to a buffer containing the current data of the state
/// - [in] `buffer_size`: Size of the buffer in bytes
///
/// # Returns
/// An `NTSTATUS` value that is `>= 0` on success and `< 0` on failure
///
/// # Assumption
/// During the runtime of the callback:
/// - `buffer` is valid for reads of size `buffer_size`
/// - the memory range of size `buffer_size` starting at `buffer` is initialized
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
    /// Subscribes to updates of a state
    ///
    /// # Arguments
    /// - [out] `subscription_handle`: Pointer to a `*mut c_void` buffer the subscription handle will be written to
    /// - [in] `state_name`: The state name
    /// - [in] `change_stamp`: The change stamp the listener has last seen
    /// - [in] `callback`: Pointer to a callback function to be called on state updates
    /// - [in] `callback_context`: Opaque pointer to arbitrary context data that is passed on to the callback
    /// - [in] `type_id`: Pointer to a GUID used as the type ID, can be a null pointer
    /// - [in] `serialization_group`: Irrelevant, can be zero
    /// - [in] `unknown`: Irrelevant, can be zero
    ///
    /// # Returns
    /// An `NTSTATUS` value that is `>= 0` on success and `< 0` on failure
    ///
    /// # Safety
    /// - `subscription_handle` must be valid for writes of `*mut c_void`
    /// - The function pointed to by `callback` must not unwind
    /// - `type_id` must either be a null pointer or point to a valid [`GUID`]
    ///
    /// # Assumption
    /// On every call of `callback(_, _, _, context, _, _)`:
    /// - `context` is the `callback_context` passed to some successful call to `RtlSubscribeWnfStateChangeNotification`
    /// - The assumptions listed under [`WnfUserCallback`] are satisfied
    pub(crate) fn RtlSubscribeWnfStateChangeNotification(
        subscription_handle: *mut *mut c_void,
        state_name: u64,
        change_stamp: u32,
        callback: WnfUserCallback,
        callback_context: *mut c_void,
        type_id: *const GUID,
        serialization_group: u32,
        unknown: u32,
    ) -> NTSTATUS;

    /// Unsubscribes from updates of a state
    ///
    /// # Arguments
    /// - [in] `subscription_handle`: The subscription handle to unsubscribe
    ///
    /// # Returns
    /// An `NTSTATUS` value that is `>= 0` on success and `< 0` on failure
    ///
    /// # Safety
    /// - `subscription_handle` must have been returned from a successful call to
    ///   `RtlSubscribeWnfStateChangeNotification`
    /// - `RtlUnsubscribeWnfStateChangeNotification` must not have been called with `subscription_handle` before
    ///
    /// # Assumptions
    /// - If `subscription_handle` was returned from a successful call of `RtlSubscribeWnfStateChangeNotification(_, _,
    ///   _, callback, callback_context, _, _, _)`, where `callback_context` is unique among all such calls, and this
    ///   function succeeds, then `callback` is not called with `callback_context` anymore.
    /// - This function is safe to call with a `subscription_handle` originating from a different thread
    pub(crate) fn RtlUnsubscribeWnfStateChangeNotification(subscription_handle: *mut c_void) -> NTSTATUS;

    /// Creates a new state
    ///
    /// # Arguments
    /// - [out] `state_name`: Pointer to a `u64` buffer the state name will be written to
    /// - [in] `name_lifetime`: The lifetime of the state; at least the following values are valid:
    ///   - `0`: "Well-known"
    ///   - `1`: "Permanent"
    ///   - `2`: "Persistent"
    ///   - `3`: "Temporary"
    /// - [in] `data_scope`: The data scope of the state; at least the following values are valid:
    ///   - `0`: "System"
    ///   - `1`: "Session"
    ///   - `2`: "User"
    ///   - `3`: "Process"
    ///   - `4`: "Machine"
    ///   - `5`: "Physical Machine"
    /// - [in] `persist_data`: Whether the state should have persistent data (`1`) or not (`0`)
    /// - [in] `type_id`: Pointer to a GUID used as the type ID, can be a null pointer
    /// - [in] `maximum_state_size`: The maximal allowed size of the state in bytes, must be between `0` and `0x1000`
    ///   (inclusive)
    /// - [in] `security_descriptor`: Pointer to a security descriptor
    ///
    /// # Returns
    /// An `NTSTATUS` value that is `>= 0` on success and `< 0` on failure
    ///
    /// # Safety
    /// - `state_name` must be valid for writes of `u64`
    /// - `type_id` must either be a null pointer or point to a valid [`GUID`]
    /// - `security_descriptor` must point to a valid security descriptor
    pub(crate) fn NtCreateWnfStateName(
        state_name: *mut u64,
        name_lifetime: u32,
        data_scope: u32,
        persist_data: u8,
        type_id: *const GUID,
        maximum_state_size: u32,
        security_descriptor: PSECURITY_DESCRIPTOR,
    ) -> NTSTATUS;

    /// Deletes a state
    ///
    /// # Arguments
    /// - [in] `state_name`: Pointer to the state name
    ///
    /// # Returns
    /// An `NTSTATUS` value that is `>= 0` on success and `< 0` on failure
    ///
    /// # Safety
    /// - `state_name` must point to a valid `u64`
    pub(crate) fn NtDeleteWnfStateName(state_name: *const u64) -> NTSTATUS;

    /// Queries the data of a state
    ///
    /// # Arguments
    /// - [in] `state_name`: Pointer to the state name
    /// - [in] `type_id`: Pointer to a GUID used as the type ID, can be a null pointer
    /// - [in] `explicit_scope`: Irrelevant, can be a null pointer
    /// - [out] `change_stamp`: Pointer to a `u32` buffer the change stamp will be written to
    /// - [out] `buffer`: Pointer to a buffer the data will be written to
    /// - [in, out] `buffer_size`: Pointer to a `u32` buffer containing the size of the buffer pointed to by `buffer` in
    ///   bytes and receiving the actual required buffer size in bytes
    ///
    /// # Returns
    /// An `NTSTATUS` value that is `>= 0` on success and `< 0` on failure
    ///
    /// # Safety
    /// - `state_name` must point to a valid `u64`
    /// - `type_id` must either be a null pointer or point to a valid [`GUID`]
    /// - `change_stamp` must be valid for writes of `u32`
    /// - `buffer` must be valid for writes of at least size `*buffer_size`
    /// - `buffer_size` must point to a valid `u32`
    /// - `buffer_size` must be valid for writes of `u32`
    ///
    /// # Assumption
    /// If this function succeeds, then the memory range of size `*buffer_size` (read after the call) starting at
    /// `buffer` is initialized.
    pub(crate) fn NtQueryWnfStateData(
        state_name: *const u64,
        type_id: *const GUID,
        explicit_scope: *const c_void,
        change_stamp: *mut u32,
        buffer: *mut c_void,
        buffer_size: *mut u32,
    ) -> NTSTATUS;

    /// Queries information about a state name
    ///
    /// The information is written into a 4-byte buffer and is always a boolean value, where `0` means `false` and `1`
    /// means `true`.
    ///
    /// # Arguments
    /// - [in] `state_name`: Pointer to the state name
    /// - [in] `name_info_class`: Tag of the class of information to obtain; at least the following values are valid:
    ///   - `0`: "State name exist"
    ///   - `1`: "Subscribers present"
    ///   - `2`: "Is quiescent"
    /// - [in] `explicit_scope`: Irrelevant, can be a null pointer
    /// - [out] `buffer`: Pointer to a `u32` buffer the information will be written to
    /// - [in] `buffer_size`: Size of the buffer in bytes, must be `4`
    ///
    /// # Returns
    /// An `NTSTATUS` value that is `>= 0` on success and `< 0` on failure
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

    /// Updates the data of a state
    ///
    /// # Arguments
    /// - [in] `state_name`: Pointer to the state name
    /// - [in] `buffer`: Pointer to a buffer the data will be read from
    /// - [in] `buffer_size`: Size of the buffer in bytes
    /// - [in] `type_id`: Pointer to a GUID used as the type ID, can be a null pointer
    /// - [in] `explicit_scope`: Irrelevant, can be a null pointer
    /// - [in] `matching_change_stamp`: The expected current change stamp of the state (only relevant if `check_stamp`
    ///   is `1`)
    /// - [in] `check_stamp`: `1` if the update should only be performed if the current change stamp equals
    ///   `matching_change_stamp`, `0` if the update should be performed regardless of the current change stamp
    ///
    /// # Returns
    /// An `NTSTATUS` value that is `>= 0` on success and `< 0` on failure
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
