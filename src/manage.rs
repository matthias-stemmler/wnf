use std::borrow::Borrow;
use std::ffi::c_void;
use std::io;

use tracing::debug;

use crate::ntdll_sys::{self, NTDLL_TARGET};
use crate::security::SecurityDescriptor;
use crate::state::{BorrowedWnfState, OwnedWnfState, RawWnfState};
use crate::state_name::{WnfDataScope, WnfStateName, WnfStateNameLifetime};
use crate::type_id::{TypeId, GUID};
use crate::BoxedSecurityDescriptor;

const MAXIMUM_STATE_SIZE: usize = 0x1000; // 4 KB

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UnspecifiedLifetime {
    _private: (),
}

impl UnspecifiedLifetime {
    fn new() -> Self {
        Self { _private: () }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UnspecifiedScope {
    _private: (),
}

impl UnspecifiedScope {
    fn new() -> Self {
        Self { _private: () }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UnspecifiedSecurityDescriptor {
    _private: (),
}

impl UnspecifiedSecurityDescriptor {
    fn new() -> Self {
        Self { _private: () }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WnfCreatableStateLifetime {
    Permanent { persist_data: bool },
    Persistent,
    Temporary,
}

impl WnfCreatableStateLifetime {
    fn persist_data(&self) -> bool {
        matches!(self, Self::Permanent { persist_data: true })
    }
}

impl From<WnfCreatableStateLifetime> for WnfStateNameLifetime {
    fn from(lifetime: WnfCreatableStateLifetime) -> Self {
        match lifetime {
            WnfCreatableStateLifetime::Permanent { .. } => WnfStateNameLifetime::Permanent,
            WnfCreatableStateLifetime::Persistent => WnfStateNameLifetime::Persistent,
            WnfCreatableStateLifetime::Temporary => WnfStateNameLifetime::Temporary,
        }
    }
}

pub trait TryIntoSecurityDescriptor {
    type IntoSecurityDescriptor: Borrow<SecurityDescriptor>;

    fn try_into_security_descriptor(self) -> io::Result<Self::IntoSecurityDescriptor>;
}

impl<SD> TryIntoSecurityDescriptor for SD
where
    SD: Borrow<SecurityDescriptor>,
{
    type IntoSecurityDescriptor = Self;

    fn try_into_security_descriptor(self) -> io::Result<Self> {
        Ok(self)
    }
}

impl TryIntoSecurityDescriptor for UnspecifiedSecurityDescriptor {
    type IntoSecurityDescriptor = BoxedSecurityDescriptor;

    fn try_into_security_descriptor(self) -> io::Result<BoxedSecurityDescriptor> {
        BoxedSecurityDescriptor::create_everyone_generic_all()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WnfStateCreation<L, S, SD> {
    // mandatory fields
    lifetime: L,
    scope: S,

    // optional fields
    maximum_state_size: Option<usize>,
    security_descriptor: SD,
    type_id: TypeId,
}

impl Default for WnfStateCreation<UnspecifiedLifetime, UnspecifiedScope, UnspecifiedSecurityDescriptor> {
    fn default() -> Self {
        Self {
            lifetime: UnspecifiedLifetime::new(),
            scope: UnspecifiedScope::new(),

            maximum_state_size: None,
            security_descriptor: UnspecifiedSecurityDescriptor::new(),
            type_id: TypeId::none(),
        }
    }
}

impl WnfStateCreation<UnspecifiedLifetime, UnspecifiedScope, UnspecifiedSecurityDescriptor> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<L, S, SD> WnfStateCreation<L, S, SD> {
    pub fn lifetime(self, lifetime: WnfCreatableStateLifetime) -> WnfStateCreation<WnfCreatableStateLifetime, S, SD> {
        WnfStateCreation {
            lifetime,

            scope: self.scope,
            security_descriptor: self.security_descriptor,
            maximum_state_size: self.maximum_state_size,
            type_id: self.type_id,
        }
    }

    pub fn scope(self, scope: WnfDataScope) -> WnfStateCreation<L, WnfDataScope, SD> {
        WnfStateCreation {
            scope,

            lifetime: self.lifetime,
            maximum_state_size: self.maximum_state_size,
            security_descriptor: self.security_descriptor,
            type_id: self.type_id,
        }
    }

    pub fn maximum_state_size(self, maximum_state_size: usize) -> WnfStateCreation<L, S, SD> {
        WnfStateCreation {
            maximum_state_size: Some(maximum_state_size),
            ..self
        }
    }

    pub fn security_descriptor<NewSD>(self, security_descriptor: NewSD) -> WnfStateCreation<L, S, NewSD>
    where
        NewSD: Borrow<SecurityDescriptor>,
    {
        WnfStateCreation {
            security_descriptor,

            lifetime: self.lifetime,
            maximum_state_size: self.maximum_state_size,
            scope: self.scope,
            type_id: self.type_id,
        }
    }

    pub fn type_id(self, type_id: impl Into<GUID>) -> WnfStateCreation<L, S, SD> {
        WnfStateCreation {
            type_id: type_id.into().into(),
            ..self
        }
    }
}

impl<SD> WnfStateCreation<WnfCreatableStateLifetime, WnfDataScope, SD>
where
    SD: TryIntoSecurityDescriptor,
{
    pub fn create_owned<T>(self) -> io::Result<OwnedWnfState<T>>
    where
        T: ?Sized,
    {
        self.create_raw().map(OwnedWnfState::from_raw)
    }

    pub fn create_static<T>(self) -> io::Result<BorrowedWnfState<'static, T>>
    where
        T: ?Sized,
    {
        self.create_raw().map(BorrowedWnfState::from_raw)
    }

    fn create_raw<T>(self) -> io::Result<RawWnfState<T>>
    where
        T: ?Sized,
    {
        RawWnfState::create(
            self.lifetime.into(),
            self.scope,
            self.lifetime.persist_data(),
            self.type_id,
            self.maximum_state_size.unwrap_or(MAXIMUM_STATE_SIZE),
            self.security_descriptor.try_into_security_descriptor()?,
        )
    }
}

impl<T> OwnedWnfState<T>
where
    T: ?Sized,
{
    pub fn create_temporary() -> io::Result<Self> {
        WnfStateCreation::new()
            .lifetime(WnfCreatableStateLifetime::Temporary)
            .scope(WnfDataScope::Machine)
            .create_owned()
    }

    pub fn delete(self) -> io::Result<()> {
        self.into_raw().delete()
    }
}

impl<T> BorrowedWnfState<'static, T>
where
    T: ?Sized,
{
    pub fn create_temporary() -> io::Result<Self> {
        WnfStateCreation::new()
            .lifetime(WnfCreatableStateLifetime::Temporary)
            .scope(WnfDataScope::Machine)
            .create_static()
    }
}

impl<T> BorrowedWnfState<'_, T>
where
    T: ?Sized,
{
    pub fn delete(self) -> io::Result<()> {
        self.raw.delete()
    }
}

impl<T> RawWnfState<T>
where
    T: ?Sized,
{
    fn create(
        name_lifetime: WnfStateNameLifetime,
        data_scope: WnfDataScope,
        persist_data: bool,
        type_id: TypeId,
        maximum_state_size: usize,
        security_descriptor: impl Borrow<SecurityDescriptor>,
    ) -> io::Result<Self> {
        let mut opaque_value = 0;

        let name_lifetime = name_lifetime as u32;
        let data_scope = data_scope as u32;
        let persist_data = persist_data as u8;
        let maximum_state_size = maximum_state_size as u32;

        let result = unsafe {
            ntdll_sys::ZwCreateWnfStateName(
                &mut opaque_value,
                name_lifetime,
                data_scope,
                persist_data,
                type_id.as_ptr(),
                maximum_state_size,
                security_descriptor.borrow() as *const SecurityDescriptor as *mut c_void,
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
                input.type_id = %type_id,
                input.maximum_state_size = maximum_state_size,
                output.state_name = %state_name,
                "ZwCreateWnfStateName",
            );

            Ok(Self::from_state_name_and_type_id(state_name, type_id))
        } else {
            debug!(
                target: NTDLL_TARGET,
                ?result,
                input.name_lifetime = name_lifetime,
                input.data_scope = data_scope,
                input.persist_data = persist_data,
                input.type_id = %type_id,
                input.maximum_state_size = maximum_state_size,
                "ZwCreateWnfStateName",
            );

            Err(io::Error::from_raw_os_error(result.0))
        }
    }

    pub(crate) fn delete(self) -> io::Result<()> {
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
}
