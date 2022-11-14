//! Methods for creating and deleting states

use std::borrow::Borrow;
use std::io;

use tracing::debug;

use crate::security::SecurityDescriptor;
use crate::state::{BorrowedState, OwnedState, RawState};
use crate::state_name::{DataScope, StateName, StateNameLifetime};
use crate::type_id::{TypeId, GUID};
use crate::{ntapi, BoxedSecurityDescriptor};

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
/// - The [`CreatableStateLifetime::Permanent`] option comes with a `persist_data` flag because that flag only applies
///   to the [`StateNameLifetime::Permanent`](crate::state_name::StateNameLifetime::Permanent) (and
///   [`StateNameLifetime::WellKnown`](crate::state_name::StateNameLifetime::WellKnown)) lifetimes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CreatableStateLifetime {
    Permanent { persist_data: bool },
    Persistent,
    Temporary,
}

impl CreatableStateLifetime {
    /// Returns whether the `persist_data` flag should be set for this [`CreatableStateLifetime`]
    fn persist_data(self) -> bool {
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

/// Trait for types that can be fallibly converted into something that can be borrowed as a [`SecurityDescriptor`]
///
/// This trait is implemented for
/// - all types that implement [`Borrow<SecurityDescriptor>`]
/// - the type [`UnspecifiedSecurityDescriptor`]
///
/// This allows the [`StateCreation::create_owned`] and [`StateCreation::create_static`] methods to be called after
/// - either setting a security descriptor explicitly through [`StateCreation::security_descriptor`]
/// - or leaving the security descriptor unspecified (in which case a default security descriptor will be used)
/// while avoiding initial creation of a default security descriptor if one is specified explicitly later.
///
/// This trait is sealed and cannot be implemented outside of `wnf`.
pub trait TryIntoSecurityDescriptor: private::Sealed {
    /// The target type of the fallible conversion
    type IntoSecurityDescriptor: Borrow<SecurityDescriptor>;

    /// Performs the fallible conversion
    ///
    /// # Errors
    /// Returns an error if the conversion fails
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

/// Builder type for creating states
///
/// You can use this type to create a state by applying the following steps:
/// 1. Create a new builder using [`StateCreation::new`]
/// 2. Configure options using the appropriate methods on [`StateCreation`]
/// 3. Call [`StateCreation::create_owned`] or [`StateCreation::create_static`] to create the state
///
/// The following options can be configured:
///
/// - [`lifetime`](StateCreation::lifetime): Mandatory
/// - [`scope`](StateCreation::scope): Mandatory
/// - [`maximum_state_size`](StateCreation::maximum_state_size): Optional, default: `0x1000`
/// - [`security_descriptor`](StateCreation::security_descriptor): Optional, default:
///   [`BoxedSecurityDescriptor::create_everyone_generic_all`]
/// - [`type_id`](StateCreation::type_id): Optional, default: none
///
/// Due to type state, the [`StateCreation::create_owned`] and [`StateCreation::create_static`] methods are only
/// available once the mandatory options have been configured.
///
/// # Example
/// ```
/// use wnf::{CreatableStateLifetime, DataScope, OwnedState, StateCreation};
///
/// let state: OwnedState<u32> = StateCreation::new()
///     .lifetime(CreatableStateLifetime::Temporary)
///     .scope(DataScope::Machine)
///     .create_owned()
///     .expect("Failed to create state");
/// ```
///
/// If you want to create multiple states from a single builder, clone the builder first:
/// ```
/// use wnf::{CreatableStateLifetime, DataScope, OwnedState, StateCreation};
///
/// let template = StateCreation::new()
///     .lifetime(CreatableStateLifetime::Temporary)
///     .scope(DataScope::Machine);
///
/// let large_state: OwnedState<u32> = template
///     .clone()
///     .maximum_state_size(0x800)
///     .create_owned()
///     .expect("Failed to create state");
///
/// let small_state: OwnedState<u32> = template
///     .maximum_state_size(0x400)
///     .create_owned()
///     .expect("Failed to create state");
/// ```
///
/// In order to quickly create a temporary machine-scoped state (e.g. for testing purposes), consider using the
/// [`OwnedState::create_temporary`] or [`BorrowedState::create_temporary`] methods.
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
    /// Creates a new [`StateCreation`] builder with no configured options
    pub fn new() -> Self {
        Self::default()
    }
}

impl<L, S, SD> StateCreation<L, S, SD> {
    /// Configures the lifetime of a [`StateCreation`] builder
    ///
    /// This is a mandatory option and must be configured before a state can be created.
    #[must_use]
    pub fn lifetime(self, lifetime: CreatableStateLifetime) -> StateCreation<CreatableStateLifetime, S, SD> {
        StateCreation {
            lifetime,

            scope: self.scope,
            security_descriptor: self.security_descriptor,
            maximum_state_size: self.maximum_state_size,
            type_id: self.type_id,
        }
    }

    /// Configures the scope of a [`StateCreation`] builder
    ///
    /// This is a mandatory option and must be configured before a state can be created.
    #[must_use]
    pub fn scope(self, scope: DataScope) -> StateCreation<L, DataScope, SD> {
        StateCreation {
            scope,

            lifetime: self.lifetime,
            maximum_state_size: self.maximum_state_size,
            security_descriptor: self.security_descriptor,
            type_id: self.type_id,
        }
    }

    /// Configures the maximum state size of a [`StateCreation`] builder
    ///
    /// If this is not configured, it defaults to `0x1000` (4 KB), which is the absolute maximum size of a state.
    #[must_use]
    pub fn maximum_state_size(self, maximum_state_size: usize) -> StateCreation<L, S, SD> {
        StateCreation {
            maximum_state_size: Some(maximum_state_size),
            ..self
        }
    }

    /// Configures the security descriptor of a [`StateCreation`] builder
    ///
    /// If this is not configured, it defaults to [`BoxedSecurityDescriptor::create_everyone_generic_all`].
    #[must_use]
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

    /// Configures the type id of a [`StateCreation`] builder
    ///
    /// If this is not configured, it defaults to no type id.
    #[must_use]
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
    /// Creates an [`OwnedState<T>`] from this [`StateCreation`]
    ///
    /// Note that the state will be deleted when the returned [`OwnedState<T>`] is dropped. You can avoid this by
    /// calling [`StateCreation::create_static`] instead, which returns a statically borrowed state.
    ///
    /// This method is only available once [`StateCreation::lifetime`] and [`StateCreation::scope`] have been called.
    ///
    /// # Errors
    /// Returns an error if creating the state fails
    pub fn create_owned<T>(self) -> io::Result<OwnedState<T>>
    where
        T: ?Sized,
    {
        self.create_raw().map(OwnedState::from_raw)
    }

    /// Creates a state from this [`StateCreation`], returning a [`BorrowsState<'static, T>`]
    ///
    /// This is equivalent to creating an owned state and immediately leaking it:
    /// ```
    /// # use wnf::{BorrowedState, CreatableStateLifetime, DataScope, StateCreation};
    /// let state: BorrowedState<'static, u32> = StateCreation::new()
    ///     .lifetime(CreatableStateLifetime::Temporary)
    ///     .scope(DataScope::Machine)
    ///     .create_owned()
    ///     .expect("Failed to create state")
    ///     .leak();
    /// ```
    ///
    /// Note that since you only obtain a statically borrowed state, it will not be deleted automatically. If that is
    /// not the desired behavior, call [`StateCreation::create_owned`] instead, which returns an owned state.
    ///
    /// This method is only available once [`StateCreation::lifetime`] and [`StateCreation::scope`] have been called.
    ///
    /// # Errors
    /// Returns an error if creating the state fails
    pub fn create_static<T>(self) -> io::Result<BorrowedState<'static, T>>
    where
        T: ?Sized,
    {
        self.create_raw().map(BorrowedState::from_raw)
    }

    /// Creates a [`RawState<T>`] from this [`StateCreation`]
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
    /// Creates an [`OwnedState<T>`] with temporary lifetime and machine scope
    ///
    /// This is a convenience method for quickly creating a state, e.g. for testing purposes.
    ///
    /// # Errors
    /// Returns an error if creating the state fails
    pub fn create_temporary() -> io::Result<Self> {
        StateCreation::new()
            .lifetime(CreatableStateLifetime::Temporary)
            .scope(DataScope::Machine)
            .create_owned()
    }

    /// Deletes this state
    ///
    /// Note that an [`OwnedState<T>`] will be deleted automatically when it is dropped, so calling this method is
    /// usually not necessary. It is useful, however, if you want to handle errors.
    ///
    /// # Errors
    /// Returns an error if deleting the state fails
    pub fn delete(self) -> io::Result<()> {
        self.into_raw().delete()
    }
}

impl<T> BorrowedState<'static, T>
where
    T: ?Sized,
{
    /// Creates a [`BorrowedState<'static, T>`] with temporary lifetime and machine scope
    ///
    /// This is a convenience method for quickly creating a state, e.g. for testing purposes.
    ///
    /// # Errors
    /// Returns an error if creating the state fails
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
    /// Deletes this state
    ///
    /// # Errors
    /// Returns an error if deleting the state fails
    pub fn delete(self) -> io::Result<()> {
        self.raw.delete()
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    /// Creates a [`RawState<T>`]
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
        let persist_data: u8 = persist_data.into();
        let maximum_state_size = maximum_state_size as u32;

        // SAFETY:
        // - The pointer in the first argument is valid for writes of `u64` because it comes from a live mutable
        //   reference
        // - The pointer in the fifth argument is either a null pointer or points to a valid `GUID` by the guarantees of
        //   `TypeId::as_ptr`
        // - The pointer in the seventh argument points to a valid security descriptor because it comes from a live
        //   reference
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

    /// Deletes this [`RawState<T>`]
    pub(crate) fn delete(self) -> io::Result<()> {
        // SAFETY:
        // The pointer points to a valid `u64` because it comes from a live reference
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

/// Making [`TryIntoSecurityDescriptor`] a sealed trait
mod private {
    use super::*;

    pub trait Sealed {}

    impl<SD> Sealed for SD where SD: Borrow<SecurityDescriptor> {}
    impl Sealed for UnspecifiedSecurityDescriptor {}
}
