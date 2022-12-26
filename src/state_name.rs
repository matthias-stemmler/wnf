//! Dealing with state names and their properties

#![deny(unsafe_code)]

use std::fmt::{self, Binary, Display, Formatter, LowerHex, Octal, UpperHex};

use num_traits::FromPrimitive;
use thiserror::Error;

/// The magic number for converting between opaque and transparent values of state names
///
/// For reference, see e.g. <https://blog.quarkslab.com/playing-with-the-windows-notification-facility-wnf.html>
const STATE_NAME_XOR_KEY: u64 = 0x41C6_4E6D_A3BC_0074;

/// The lifetime of a state
///
/// This property of a state controls at what point in time it is automatically deleted as well as if and how it is
/// persisted.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, Hash, PartialEq)]
#[repr(u8)]
pub enum StateLifetime {
    /// Lifetime of a "well-known" state
    ///
    /// A state with this lifetime cannot be created or deleted through the WNF API, but instead is provisioned with
    /// the system. It lives forever.
    ///
    /// It is persisted in the Windows registry under the key
    /// `HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Control\Notifications`
    WellKnown = 0,

    /// Lifetime of a "permanent"  state
    ///
    /// A state with this lifetime can be created and deleted through the WNF API at any time and is never deleted
    /// automatically.
    ///
    /// Creating a state with this lifetime requires the `SeCreatePermanentPrivilege` privilege.
    ///
    /// It is persisted in the Windows registry under the key
    /// `HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Notifications`
    Permanent = 1,

    /// Lifetime of a "persistent" state (also known as "volatile")
    ///
    /// A state with this lifetime can be created and deleted through the WNF API at any time and is automatically
    /// deleted on system reboot.
    ///
    /// Creating a state with this lifetime requires the `SeCreatePermanentPrivilege` privilege.
    ///
    /// The name "persistent" is meant in relation to a temporary state name because it is persisted beyond the
    /// lifetime of the process it was created from. The alternative name "volatile" is meant in relation to a
    /// permanent state name because it is deleted on system reboot.
    ///
    /// It is persisted in the Windows registry under the key
    /// `HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion\VolatileNotifications`
    Persistent = 2,

    /// Lifetime of a "temporary" state
    ///
    /// A state with this lifetime can be created and deleted through the WNF API at any time and is automatically
    /// deleted when the process it was created from exits.
    ///
    /// It is not persisted in the Windows registry.
    Temporary = 3,
}

/// The data scope of a state
///
/// This property of a state controls whether it maintains multiple instances of its data that are scoped in different
/// ways.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, Hash, PartialEq)]
#[repr(u8)]
pub enum DataScope {
    /// "System" data scope
    System = 0,

    /// "Session" data scope
    Session = 1,

    /// "User" data scope
    User = 2,

    /// "Process" data scope
    Process = 3,

    /// "Machine" data scope
    ///
    /// This seems to be the widest available data scope that all reverse engineering resources agree on. It is a good
    /// default choice if you don't care about data scope.
    Machine = 4,

    /// "Physical Machine" data scope
    ///
    /// This is only mentioned by some reverse engineering resources, not all of them. However, there exist
    /// (well-known) state names with this data scope.
    PhysicalMachine = 5,
}

/// The descriptor of a state name
///
/// This contains the properties of a [`StateName`] that are encoded in the bits of its transparent value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StateNameDescriptor {
    /// WNF version number, currently always `1`
    pub version: u8,

    /// Lifetime of the state name
    pub lifetime: StateLifetime,

    /// Data scope of the state name
    pub data_scope: DataScope,

    /// Whether the state data (not the state name itself) are persisted across system reboots
    ///
    /// This only applies to state names with the [`StateLifetime::WellKnown`] or
    /// [`StateLifetime::Permanent`] lifetimes. It is always `false` for state names with other lifetimes.
    pub is_permanent: bool,

    /// Unique sequence number of the state name
    pub unique_id: u32,

    /// Owner tag of the state name
    ///
    /// This only applies to state names with the [`StateLifetime::WellKnown`] lifetime. It is always `0` for
    /// state names with other lifetimes.
    pub owner_tag: u32,
}

/// A state name
///
/// A state name is usually represented by its "opaque value", which is a 64-bit integer. This opaque value can be
/// converted to a "transparent" value by XOR'ing with the magic number `0x41C64E6DA3BC0074`. This transparent value
/// encodes certain properties of the state name in its bits. The set of these properties is represented by the
/// [`StateNameDescriptor`] type. Use the provided [`TryFrom`]/[`TryInto`] implementations to convert between a
/// [`StateName`] (represented by its opaque value) and the corresponding [`StateNameDescriptor`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StateName {
    opaque_value: u64,
}

impl StateName {
    /// Creates a [`StateName`] from the given opaque value
    pub const fn from_opaque_value(opaque_value: u64) -> Self {
        Self { opaque_value }
    }

    /// Returns the opaque value of this [`StateName`]
    pub const fn opaque_value(self) -> u64 {
        self.opaque_value
    }
}

impl From<u64> for StateName {
    fn from(opaque_value: u64) -> Self {
        Self::from_opaque_value(opaque_value)
    }
}

impl From<StateName> for u64 {
    fn from(state_name: StateName) -> Self {
        state_name.opaque_value()
    }
}

impl PartialEq<u64> for StateName {
    fn eq(&self, other: &u64) -> bool {
        self.opaque_value == *other
    }
}

impl PartialEq<StateName> for u64 {
    fn eq(&self, other: &StateName) -> bool {
        *self == other.opaque_value
    }
}

impl Display for StateName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self:X}")
    }
}

impl UpperHex for StateName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018X}", self.opaque_value)
    }
}

impl LowerHex for StateName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.opaque_value)
    }
}

impl Octal for StateName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:#024o}", self.opaque_value)
    }
}

impl Binary for StateName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:#066b}", self.opaque_value)
    }
}

impl TryFrom<StateNameDescriptor> for StateName {
    type Error = StateNameFromDescriptorError;

    fn try_from(descriptor: StateNameDescriptor) -> Result<Self, Self::Error> {
        if descriptor.version >= (1 << 4) {
            return Err(StateNameFromDescriptorError::InvalidVersion(descriptor.version));
        }

        if descriptor.unique_id >= (1 << 21) {
            return Err(StateNameFromDescriptorError::InvalidUniqueId(descriptor.unique_id));
        }

        let transparent_value = u64::from(descriptor.version)
            + ((descriptor.lifetime as u64) << 4)
            + ((descriptor.data_scope as u64) << 6)
            + ((u64::from(descriptor.is_permanent)) << 10)
            + ((u64::from(descriptor.unique_id)) << 11)
            + ((u64::from(descriptor.owner_tag)) << 32);

        let opaque_value = transparent_value ^ STATE_NAME_XOR_KEY;

        Ok(Self { opaque_value })
    }
}

impl TryFrom<StateName> for StateNameDescriptor {
    type Error = StateNameDescriptorFromStateNameError;

    fn try_from(state_name: StateName) -> Result<Self, Self::Error> {
        let transparent_value = state_name.opaque_value ^ STATE_NAME_XOR_KEY;

        let lifetime_value = ((transparent_value >> 4) & 0b11) as u8;
        let data_scope_value = ((transparent_value >> 6) & 0b1111) as u8;

        Ok(Self {
            version: (transparent_value & 0b1111) as u8,
            // Since `lifetime_value <= 3`, this always succeeds
            lifetime: StateLifetime::from_u8(lifetime_value).unwrap(),
            data_scope: DataScope::from_u8(data_scope_value).ok_or(
                StateNameDescriptorFromStateNameError::InvalidDataScope(data_scope_value),
            )?,
            is_permanent: transparent_value & (1 << 10) != 0,
            unique_id: ((transparent_value >> 11) & 0x001F_FFFF) as u32,
            owner_tag: (transparent_value >> 32) as u32,
        })
    }
}

/// An error converting a [`StateNameDescriptor`] into a [`StateName`]
#[derive(Clone, Copy, Debug, Error, Eq, Hash, PartialEq)]
pub enum StateNameFromDescriptorError {
    /// The [`StateNameDescriptor::version`] is invalid (must be less than `2^4`)
    #[error("invalid version: {0}")]
    InvalidVersion(u8),

    /// The [`StateNameDescriptor::unique_id`] is invalid (must be less than `2^21`)
    #[error("invalid unique id: {0}")]
    InvalidUniqueId(u32),
}

/// An error converting a [`StateName`] into a [`StateNameDescriptor`]
#[derive(Clone, Copy, Debug, Error, Eq, Hash, PartialEq)]
pub enum StateNameDescriptorFromStateNameError {
    /// The data scope encoded in the state name is invalid (must be in `0..=5`)
    #[error("invalid data scope: {0}")]
    InvalidDataScope(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_STATE_NAME: StateName = StateName::from_opaque_value(0x0D83_063E_A3BE_5075);
    const SAMPLE_DESCRIPTOR: StateNameDescriptor = StateNameDescriptor {
        version: 1,
        lifetime: StateLifetime::WellKnown,
        data_scope: DataScope::System,
        is_permanent: false,
        unique_id: 0x0000_004A,
        owner_tag: 0x4C45_4853,
    };

    #[test]
    fn state_name_into_descriptor_success() {
        let result: Result<StateNameDescriptor, _> = SAMPLE_STATE_NAME.try_into();

        assert_eq!(result, Ok(SAMPLE_DESCRIPTOR));
    }

    #[test]
    fn state_name_into_descriptor_invalid_data_scope() {
        let opaque_value = 0x0D83_063E_A3BE_51F5; // this is `SAMPLE_STATE_NAME` with data scope set to 0x06

        let result: Result<StateNameDescriptor, _> = StateName::from_opaque_value(opaque_value).try_into();

        assert_eq!(
            result,
            Err(StateNameDescriptorFromStateNameError::InvalidDataScope(0x06))
        );
    }

    #[test]
    fn descriptor_into_state_name_success() {
        let result: Result<StateName, _> = SAMPLE_DESCRIPTOR.try_into();

        assert_eq!(result, Ok(SAMPLE_STATE_NAME));
    }

    #[test]
    fn descriptor_into_state_name_invalid_version() {
        let descriptor = StateNameDescriptor {
            version: 1 << 4,
            ..SAMPLE_DESCRIPTOR
        };

        let result: Result<StateName, _> = descriptor.try_into();

        assert_eq!(result, Err(StateNameFromDescriptorError::InvalidVersion(1 << 4)));
    }

    #[test]
    fn descriptor_into_state_name_invalid_unique_id() {
        let descriptor = StateNameDescriptor {
            unique_id: 1 << 21,
            ..SAMPLE_DESCRIPTOR
        };

        let result: Result<StateName, _> = descriptor.try_into();

        assert_eq!(result, Err(StateNameFromDescriptorError::InvalidUniqueId(1 << 21)));
    }

    #[test]
    fn state_name_display() {
        assert_eq!(SAMPLE_STATE_NAME.to_string(), "0x0D83063EA3BE5075");
    }

    #[test]
    fn state_name_upper_hex() {
        assert_eq!(format!("{SAMPLE_STATE_NAME:X}"), "0x0D83063EA3BE5075");
    }

    #[test]
    fn state_name_lower_hex() {
        assert_eq!(format!("{SAMPLE_STATE_NAME:x}"), "0x0d83063ea3be5075");
    }

    #[test]
    fn state_name_octal() {
        assert_eq!(format!("{SAMPLE_STATE_NAME:o}"), "0o0066030143724357450165");
    }

    #[test]
    fn state_name_binary() {
        assert_eq!(
            format!("{SAMPLE_STATE_NAME:b}"),
            "0b0000110110000011000001100011111010100011101111100101000001110101"
        );
    }
}
