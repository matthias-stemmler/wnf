//! Methods for creating and deleting states

use std::borrow::Borrow;
use std::fmt::{self, Debug, Formatter};
use std::io;

use tracing::debug;

use crate::ntapi;
use crate::security::{BoxedSecurityDescriptor, SecurityDescriptor};
use crate::state::{BorrowedState, OwnedState, RawState};
use crate::state_name::{DataScope, StateLifetime, StateName};
use crate::type_id::{TypeId, GUID};

/// The maximum size of a state in bytes
///
/// The maximum size of a state can be specified upon creation of the state and can be anything between `0` and
/// `4 KB`, which is the value of this constant. This value is also used as the default value when the maximum state
/// size is not specified.
pub const MAXIMUM_STATE_SIZE: usize = 0x1000;

/// A marker type for an unspecified lifetime when creating a state
///
/// The lifetime of a state must be specified upon its creation. When creating a state via a
/// [`StateCreation`], this is used as a type parameter to indicate that the lifetime has not been specified yet.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UnspecifiedLifetime {
    _private: (),
}

impl UnspecifiedLifetime {
    const fn new() -> Self {
        Self { _private: () }
    }
}

impl Debug for UnspecifiedLifetime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Hide the `_private` field
        f.debug_struct("UnspecifiedLifetime").finish()
    }
}

/// A marker type for an unspecified scope when creating a state
///
/// The scope of a state must be specified upon its creation. When creating a state via a [`StateCreation`],
/// this is used as a type parameter to indicate that the scope has not been specified yet.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UnspecifiedScope {
    _private: (),
}

impl UnspecifiedScope {
    const fn new() -> Self {
        Self { _private: () }
    }
}

impl Debug for UnspecifiedScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Hide the `_private` field
        f.debug_struct("UnspecifiedScope").finish()
    }
}

/// A marker type for an unspecified security descriptor when creating a state
///
/// The security descriptor of a state can optionally be specified upon its creation. When creating a state via
/// a [`StateCreation`], this is used as a type parameter to indicate that no security descriptor has been specified.
/// In this case, a default security descriptor (see [`BoxedSecurityDescriptor::create_everyone_generic_all`]) will be
/// used.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UnspecifiedSecurityDescriptor {
    _private: (),
}

impl UnspecifiedSecurityDescriptor {
    const fn new() -> Self {
        Self { _private: () }
    }
}

impl Debug for UnspecifiedSecurityDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Hide the `_private` field
        f.debug_struct("UnspecifiedSecurityDescriptor").finish()
    }
}

/// The lifetime of a state when specified upon creation
///
/// This is different from a [`StateLifetime`] in two ways:
/// - It does not include an equivalent of the [`StateLifetime::WellKnown`] lifetime because states with that lifetime
///   are provisioned with the system and cannot be created.
/// - The [`CreatableStateLifetime::Permanent`] option comes with a `persist_data` flag because that flag only applies
///   to the [`StateLifetime::Permanent`](crate::state_name::StateLifetime::Permanent) (and
///   [`StateLifetime::WellKnown`](crate::state_name::StateLifetime::WellKnown)) lifetimes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CreatableStateLifetime {
    /// Lifetime of a *permanent* state, see [`StateLifetime::Permanent`]
    Permanent {
        /// Whether the state data (not the state itself) are persisted across system reboots
        persist_data: bool,
    },

    /// Lifetime of a *persistent* state (also known as *volatile*), see [`StateLifetime::Persistent`]
    Persistent,

    /// Lifetime of a *temporary* state, see [`StateLifetime::Temporary`]
    Temporary,
}

impl CreatableStateLifetime {
    /// Returns whether the `persist_data` flag should be set for this [`CreatableStateLifetime`]
    const fn persist_data(self) -> bool {
        matches!(self, Self::Permanent { persist_data: true })
    }
}

impl From<CreatableStateLifetime> for StateLifetime {
    fn from(lifetime: CreatableStateLifetime) -> Self {
        match lifetime {
            CreatableStateLifetime::Permanent { .. } => StateLifetime::Permanent,
            CreatableStateLifetime::Persistent => StateLifetime::Persistent,
            CreatableStateLifetime::Temporary => StateLifetime::Temporary,
        }
    }
}

/// A trait for types that can be fallibly converted into a security descriptor
///
/// Since [`SecurityDescriptor`] is an opaque type, this does not mean (fallibly) converting into an actual
/// [`SecurityDescriptor`] but fallibly converting into some type that can be borrowed as a [`SecurityDescriptor`].
///
/// This trait is implemented for
/// - all types that implement [`Borrow<SecurityDescriptor>`]
/// - the type [`UnspecifiedSecurityDescriptor`]
///
/// This allows the [`StateCreation::create_owned`] and [`StateCreation::create_static`] methods to be called after
/// - either setting a security descriptor explicitly through [`StateCreation::security_descriptor`]
/// - or leaving the security descriptor unspecified (in which case a default security descriptor will be used) while
///   avoiding initial creation of a default security descriptor if one is specified explicitly later.
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

/// A builder type for creating states
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
/// - [`security_descriptor`](StateCreation::security_descriptor): Optional, default: see
///   [`BoxedSecurityDescriptor::create_everyone_generic_all`]
/// - [`type_id`](StateCreation::type_id): Optional, default: none
///
/// Note that the [`StateCreation::create_owned`] and [`StateCreation::create_static`] methods are only available once
/// the mandatory options have been configured.
///
/// # Example
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use wnf::{CreatableStateLifetime, DataScope, OwnedState, StateCreation};
///
/// let state: OwnedState<u32> = StateCreation::new()
///     .lifetime(CreatableStateLifetime::Temporary)
///     .scope(DataScope::Machine)
///     .create_owned()?;
/// # Ok(()) }
/// ```
///
/// If you want to create multiple states from a single builder, clone the builder first:
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use wnf::{CreatableStateLifetime, DataScope, OwnedState, StateCreation};
///
/// let template = StateCreation::new()
///     .lifetime(CreatableStateLifetime::Temporary)
///     .scope(DataScope::Machine);
///
/// let large_state: OwnedState<u32> = template.clone().maximum_state_size(0x800).create_owned()?;
///
/// let small_state: OwnedState<u32> = template.maximum_state_size(0x400).create_owned()?;
/// # Ok(()) }
/// ```
///
/// In order to quickly create a temporary machine-scoped state (e.g. for testing purposes), consider using the
/// [`OwnedState::create_temporary`] or [`BorrowedState::create_temporary`] methods.
///
/// Note that a newly created state is initialized with data of size zero. This means that unless the data type `T` is
/// zero-sized or a slice type, you need to update the state data with a value of type `T` before querying it for the
/// first time.
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
        Self::new()
    }
}

impl StateCreation<UnspecifiedLifetime, UnspecifiedScope, UnspecifiedSecurityDescriptor> {
    /// Creates a new [`StateCreation`] builder with no configured options
    pub const fn new() -> Self {
        Self {
            lifetime: UnspecifiedLifetime::new(),
            scope: UnspecifiedScope::new(),

            maximum_state_size: None,
            security_descriptor: UnspecifiedSecurityDescriptor::new(),
            type_id: TypeId::none(),
        }
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

    /// Creates a state from this [`StateCreation`], returning a [`BorrowedState<'static, T>`](BorrowedState)
    ///
    /// This is equivalent to creating an owned state and immediately leaking it:
    /// ```
    /// # use wnf::{BorrowedState, CreatableStateLifetime, DataScope, StateCreation};
    /// #
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let state: BorrowedState<'static, u32> = StateCreation::new()
    ///     .lifetime(CreatableStateLifetime::Temporary)
    ///     .scope(DataScope::Machine)
    ///     .create_owned()?
    ///     .leak();
    /// # Ok(()) }
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
    /// For more precise control over the created state, use the [`StateCreation`] builder.
    ///
    /// Note that a newly created state is initialized with data of size zero. This means that unless the data type `T`
    /// is zero-sized or a slice type, you need to update the state data with a value of type `T` before querying it
    /// for the first time.
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
    /// Creates a [`BorrowedState<'static, T>`](BorrowedState::create_temporary) with temporary lifetime and machine
    /// scope
    ///
    /// This is a convenience method for quickly creating a state, e.g. for testing purposes.
    /// For more precise control over the created state, use the [`StateCreation`] builder.
    ///
    /// Note that a newly created state is initialized with data of size zero. This means that unless the data type `T`
    /// is zero-sized or a slice type, you need to update the state data with a value of type `T` before querying it
    /// for the first time.
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
    /// Returns an error if deleting the state fails
    pub fn delete(self) -> io::Result<()> {
        self.raw.delete()
    }
}

impl<T> RawState<T>
where
    T: ?Sized,
{
    /// Creates a state
    fn create(
        name_lifetime: StateLifetime,
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

    /// Deletes this state
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
