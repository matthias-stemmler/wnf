//! Methods for creating and deleting states

use std::borrow::Borrow;
use std::io;

use tracing::debug;

use crate::ntapi;
use crate::security::SecurityDescriptor;
use crate::state::{BorrowedState, OwnedState, RawState};
use crate::state_name::{DataScope, StateName, StateNameLifetime};
use crate::type_id::{TypeId, GUID};
use crate::BoxedSecurityDescriptor;

/// The maximum size of a state in bytes
///
/// The maximum size of a state can be specified upon creation of the state and can be anything between `0` and
/// `4KB`, which is the value of this constant. It is also used as the default value when the maximum state size is not
/// specified.
pub const MAXIMUM_STATE_SIZE: usize = 0x1000;

/// Marker type for an unspecified lifetime when creating a state
///
/// The lifetime of a state must be specified upon its creation. When creating a state via a
/// [`StateCreation`], this is used as a type parameter to indicate that the lifetime has not been specified yet.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UnspecifiedLifetime {
    _private: (),
}

impl UnspecifiedLifetime {
    fn new() -> Self {
        Self { _private: () }
    }
}

/// Marker type for an unspecified scope when creating a state
///
/// The scope of a state must be specified upon its creation. When creating a state via a [`StateCreation`],
/// this is used as a type parameter to indicate that the scope has not been specified yet.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UnspecifiedScope {
    _private: (),
}

impl UnspecifiedScope {
    fn new() -> Self {
        Self { _private: () }
    }
}

/// Marker type for an unspecified security descriptor when creating a state
///
/// The security descriptor of a state can optionally be specified upon its creation. When creating a state via
/// a [`StateCreation`], this is used as a type parameter to indicate that no security descriptor has been specified.
/// In this case, a default security descriptor (see [`BoxedSecurityDescriptor::create_everyone_generic_all`]) will be
/// used.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UnspecifiedSecurityDescriptor {
    _private: (),
}

impl UnspecifiedSecurityDescriptor {
    fn new() -> Self {
        Self { _private: () }
    }
}

/// The lifetime of a state when specified upon creation
///
/// This is different from a [`StateNameLifetime`] in two ways:
/// - It does not include an equivalent of the [`StateNameLifetime::WellKnown`] lifetime because states with that
///   lifetime are provisioned with the system and cannot be created.
/// - The [`CreatableStateLifetime::Permanent`] option comes with a `persist_data` flag because that flag only
///   applies to the [`StateNameLifetime::Permant`] (and [`StateNameLifetime::WellKnown`]) lifetimes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CreatableStateLifetime {
    Permanent { persist_data: bool },
    Persistent,
    Temporary,
}

impl CreatableStateLifetime {
    fn persist_data(&self) -> bool {
        matches!(self, Self::Permanent { persist_data: true })
    }
}

impl From<CreatableStateLifetime> for StateNameLifetime {
    fn from(lifetime: CreatableStateLifetime) -> Self {
        match lifetime {
            CreatableStateLifetime::Permanent { .. } => StateNameLifetime::Permanent,
            CreatableStateLifetime::Persistent => StateNameLifetime::Persistent,
            CreatableStateLifetime::Temporary => StateNameLifetime::Temporary,
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
pub struct StateCreation<L, S, SD> {
    // mandatory fields
    lifetime: L,
    scope: S,

    // optional fields
    maximum_state_size: Option<usize>,
    security_descriptor: SD,
    type_id: TypeId,
}

impl Default for StateCreation<UnspecifiedLifetime, UnspecifiedScope, UnspecifiedSecurityDescriptor> {
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

impl StateCreation<UnspecifiedLifetime, UnspecifiedScope, UnspecifiedSecurityDescriptor> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<L, S, SD> StateCreation<L, S, SD> {
    pub fn lifetime(self, lifetime: CreatableStateLifetime) -> StateCreation<CreatableStateLifetime, S, SD> {
        StateCreation {
            lifetime,

            scope: self.scope,
            security_descriptor: self.security_descriptor,
            maximum_state_size: self.maximum_state_size,
            type_id: self.type_id,
        }
    }

    pub fn scope(self, scope: DataScope) -> StateCreation<L, DataScope, SD> {
        StateCreation {
            scope,

            lifetime: self.lifetime,
            maximum_state_size: self.maximum_state_size,
            security_descriptor: self.security_descriptor,
            type_id: self.type_id,
        }
    }

    pub fn maximum_state_size(self, maximum_state_size: usize) -> StateCreation<L, S, SD> {
        StateCreation {
            maximum_state_size: Some(maximum_state_size),
            ..self
        }
    }

    pub fn security_descriptor<NewSD>(self, security_descriptor: NewSD) -> StateCreation<L, S, NewSD>
    where
        NewSD: Borrow<SecurityDescriptor>,
    {
        StateCreation {
            security_descriptor,

            lifetime: self.lifetime,
            maximum_state_size: self.maximum_state_size,
            scope: self.scope,
            type_id: self.type_id,
        }
    }

    pub fn type_id(self, type_id: impl Into<GUID>) -> StateCreation<L, S, SD> {
        StateCreation {
            type_id: type_id.into().into(),
            ..self
        }
    }
}

impl<SD> StateCreation<CreatableStateLifetime, DataScope, SD>
where
    SD: TryIntoSecurityDescriptor,
{
    pub fn create_owned<T>(self) -> io::Result<OwnedState<T>>
    where
        T: ?Sized,
    {
        self.create_raw().map(OwnedState::from_raw)
    }

    pub fn create_static<T>(self) -> io::Result<BorrowedState<'static, T>>
    where
        T: ?Sized,
    {
        self.create_raw().map(BorrowedState::from_raw)
    }

    fn create_raw<T>(self) -> io::Result<RawState<T>>
    where
        T: ?Sized,
    {
        RawState::create(
            self.lifetime.into(),
            self.scope,
            self.lifetime.persist_data(),
            self.type_id,
            self.maximum_state_size.unwrap_or(MAXIMUM_STATE_SIZE),
            self.security_descriptor.try_into_security_descriptor()?,
        )
    }
}

impl<T> OwnedState<T>
where
    T: ?Sized,
{
    pub fn create_temporary() -> io::Result<Self> {
        StateCreation::new()
            .lifetime(CreatableStateLifetime::Temporary)
            .scope(DataScope::Machine)
            .create_owned()
    }

    pub fn delete(self) -> io::Result<()> {
        self.into_raw().delete()
    }
}

impl<T> BorrowedState<'static, T>
where
    T: ?Sized,
{
    pub fn create_temporary() -> io::Result<Self> {
        StateCreation::new()
            .lifetime(CreatableStateLifetime::Temporary)
            .scope(DataScope::Machine)
            .create_static()
    }
}

impl<T> BorrowedState<'_, T>
where
    T: ?Sized,
{
    pub fn delete(self) -> io::Result<()> {
        self.raw.delete()
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    fn create(
        name_lifetime: StateNameLifetime,
        data_scope: DataScope,
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
            ntapi::NtCreateWnfStateName(
                &mut opaque_value,
                name_lifetime,
                data_scope,
                persist_data,
                type_id.as_ptr(),
                maximum_state_size,
                security_descriptor.borrow().as_ptr(),
            )
        };

        if result.is_ok() {
            let state_name = StateName::from_opaque_value(opaque_value);

            debug!(
                target: ntapi::TRACING_TARGET,
                ?result,
                input.name_lifetime = name_lifetime,
                input.data_scope = data_scope,
                input.persist_data = persist_data,
                input.type_id = %type_id,
                input.maximum_state_size = maximum_state_size,
                output.state_name = %state_name,
                "NtCreateWnfStateName",
            );

            Ok(Self::from_state_name_and_type_id(state_name, type_id))
        } else {
            debug!(
                target: ntapi::TRACING_TARGET,
                ?result,
                input.name_lifetime = name_lifetime,
                input.data_scope = data_scope,
                input.persist_data = persist_data,
                input.type_id = %type_id,
                input.maximum_state_size = maximum_state_size,
                "NtCreateWnfStateName",
            );

            Err(io::Error::from_raw_os_error(result.0))
        }
    }

    pub(crate) fn delete(self) -> io::Result<()> {
        let result = unsafe { ntapi::NtDeleteWnfStateName(&self.state_name.opaque_value()) };

        debug!(
            target: ntapi::TRACING_TARGET,
            ?result,
            input.state_name = %self.state_name,
            "NtDeleteWnfStateName",
        );

        result.ok()?;
        Ok(())
    }
}
