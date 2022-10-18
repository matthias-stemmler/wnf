//! Dealing with WNF state names and their properties

#![deny(unsafe_code)]

use std::fmt;
use std::fmt::{Display, Formatter};

use num_traits::FromPrimitive;
use thiserror::Error;

/// This is the magic number used by WNF to convert between the opaque value of a WNF state name and its corresponding
/// transparent value that contains information about the state name encoded into its bits.
///
/// For reference, see e.g. [https://blog.quarkslab.com/playing-with-the-windows-notification-facility-wnf.html]
const STATE_NAME_XOR_KEY: u64 = 0x41C64E6DA3BC0074;

/// Lifetime of a WNF state name
///
/// This property of a WNF state name controls at what point in time the corresponding WNF state is automatically
/// deleted as well as if and how the state name is persisted.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, Hash, PartialEq)]
#[repr(u8)]
pub enum WnfStateNameLifetime {
    /// Lifetime of a "well-known" WNF state name
    ///
    /// A state name with this lifetime cannot be created or deleted through the WNF API, but instead is provisioned
    /// with the system. It lives forever.
    ///
    /// It is persisted in the Windows registry under the key
    /// `HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Control\Notifications`
    WellKnown = 0,

    /// Lifetime of a "permanent" WNF state name
    ///
    /// A state name with this lifetime can be created and deleted through the WNF API at any time, but is never deleted
    /// automatically.
    ///
    /// Creating a state name with this lifetime requires the `SeCreatePermanentPrivilege` privilege.
    ///
    /// It is persisted in the Windows registry under the key
    /// `HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Notifications`
    Permanent = 1,

    /// Lifetime of a "persistent" WNF state name (also known as "volatile")
    ///
    /// A state name with this lifetime can be created and deleted through the WNF API at any time and is automatically
    /// deleted on system reboot.
    ///
    /// Creating a state name with this lifetime requires the `SeCreatePermanentPrivilege` privilege.
    ///
    /// The name "persistent" is meant in relation to a temporary state name because it is persisted beyond the lifetime
    /// of the process it was created from. The alternative name "volatile" is meant in relation to a permanent state
    /// name because it is deleted on system reboot.
    ///
    /// It is persisted in the Windows registry under the key
    /// `HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion\VolatileNotifications`
    Persistent = 2,

    /// Lifetime of a "temporary" WNF state name
    ///
    /// A state name with this lifetime can be created and deleted through the WNF API at any time and is automatically
    /// deleted when the process it was created from exits.
    ///
    /// It is not persisted in the Windows registry.
    Temporary = 3,
}

/// Data scope of a WNF state name
///
/// This property of a WNF state name controls whether the corresponding WNF state maintains multiple instances of its
/// data that are scoped in different ways.
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, Hash, PartialEq)]
#[repr(u8)]
pub enum WnfDataScope {
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
    /// This is only mentioned by some reverse engineering resources, not all of them. However, there exist (well-known)
    /// state names with this data scope.
    PhysicalMachine = 5,
}

/// Descriptor of a WNF state name
///
/// This contains the properties of a [`WnfStateName`] that are encoded in the bits of its transparent value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WnfStateNameDescriptor {
    /// WNF version number, currently always `1`
    pub version: u8,

    /// Lifetime of the WNF state name
    pub lifetime: WnfStateNameLifetime,

    /// Data scope of the WNF state name
    pub data_scope: WnfDataScope,

    /// Whether the WNF state data (not the state name itself) are persisted across system reboots
    ///
    /// This only applies to WNF state names with the [`WnfStateNameLifetime::WellKnown`] or
    /// [`WnfStateNameLifetime::Permanent`] lifetimes. It is always `false` for state names with other lifetimes.
    pub is_permanent: bool,

    /// Unique sequence number of the WNF state name
    pub unique_id: u32,

    /// Owner tag of the WNF state name
    ///
    /// This only applies to WNF state names with the [`WnfStateNameLifetime::WellKnown`] lifetime. It is always `0` for
    /// state names with other lifetimes.
    pub owner_tag: u32,
}

/// A WNF state name
///
/// A WNF state name is usually represented by its "opaque value", which is a 64-bit integer. This opaque value can be
/// converted to a "transparent" value by XOR'ing with the magic number `0x41C64E6DA3BC0074`. This transparent value
/// encodes certain properties of the WNF state name in its bits. The set of these properties is represented by the
/// [`WnfStateNameDescriptor`] type. Use the provided [`TryFrom`]/[`TryInto`] implementations to convert between a
/// [`WnfStateName`] (represented by its opaque value) and the corresponding [`WnfStateNameDescriptor`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WnfStateName {
    opaque_value: u64,
}

impl WnfStateName {
    /// Creates a [`WnfStateName`] from the given opaque value
    pub const fn from_opaque_value(opaque_value: u64) -> Self {
        Self { opaque_value }
    }

    /// Returns the opaque value of this [`WnfStateName`]
    pub const fn opaque_value(self) -> u64 {
        self.opaque_value
    }
}

impl From<u64> for WnfStateName {
    fn from(opaque_value: u64) -> Self {
        Self::from_opaque_value(opaque_value)
    }
}

impl TryFrom<WnfStateNameDescriptor> for WnfStateName {
    type Error = WnfStateNameFromDescriptorError;

    fn try_from(descriptor: WnfStateNameDescriptor) -> Result<Self, Self::Error> {
        if descriptor.version >= (1 << 4) {
            return Err(WnfStateNameFromDescriptorError::InvalidVersion(descriptor.version));
        }

        if descriptor.unique_id >= (1 << 21) {
            return Err(WnfStateNameFromDescriptorError::InvalidUniqueId(descriptor.unique_id));
        }

        let transparent_value = descriptor.version as u64
            + ((descriptor.lifetime as u64) << 4)
            + ((descriptor.data_scope as u64) << 6)
            + ((descriptor.is_permanent as u64) << 10)
            + ((descriptor.unique_id as u64) << 11)
            + ((descriptor.owner_tag as u64) << 32);

        let opaque_value = transparent_value ^ STATE_NAME_XOR_KEY;

        Ok(Self { opaque_value })
    }
}

impl TryFrom<WnfStateName> for WnfStateNameDescriptor {
    type Error = WnfStateNameDescriptorFromStateNameError;

    fn try_from(state_name: WnfStateName) -> Result<Self, Self::Error> {
        let transparent_value = state_name.opaque_value ^ STATE_NAME_XOR_KEY;

        let lifetime_value = ((transparent_value >> 4) & 0b11) as u8;
        let data_scope_value = ((transparent_value >> 6) & 0b1111) as u8;

        Ok(Self {
            version: (transparent_value & 0b1111) as u8,
            // Since `lifetime_value <= 3`, this always succeeds
            lifetime: WnfStateNameLifetime::from_u8(lifetime_value).unwrap(),
            data_scope: WnfDataScope::from_u8(data_scope_value).ok_or(
                WnfStateNameDescriptorFromStateNameError::InvalidDataScope(data_scope_value),
            )?,
            is_permanent: transparent_value & (1 << 10) != 0,
            unique_id: ((transparent_value >> 11) & 0x001FFFFF) as u32,
            owner_tag: (transparent_value >> 32) as u32,
        })
    }
}

impl Display for WnfStateName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:#010x}", self.opaque_value)
    }
}

/// Error converting a [`WnfStateNameDescriptor`] into a [`WnfStateName`]
#[derive(Debug, Error, Eq, PartialEq)]
pub enum WnfStateNameFromDescriptorError {
    #[error("invalid version: {0}")]
    InvalidVersion(u8),

    #[error("invalid unique id: {0}")]
    InvalidUniqueId(u32),
}

/// Error converting a [`WnfStateName`] into a [`WnfStateNameDescriptor`]
#[derive(Debug, Error, Eq, PartialEq)]
pub enum WnfStateNameDescriptorFromStateNameError {
    #[error("invalid data scope: {0}")]
    InvalidDataScope(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_STATE_NAME: WnfStateName = WnfStateName::from_opaque_value(0x0D83063EA3BE5075);
    const SAMPLE_DESCRIPTOR: WnfStateNameDescriptor = WnfStateNameDescriptor {
        version: 1,
        lifetime: WnfStateNameLifetime::WellKnown,
        data_scope: WnfDataScope::System,
        is_permanent: false,
        unique_id: 0x00004A,
        owner_tag: 0x4C454853,
    };

    #[test]
    fn state_name_into_descriptor_success() {
        let result: Result<WnfStateNameDescriptor, _> = SAMPLE_STATE_NAME.try_into();

        assert_eq!(result, Ok(SAMPLE_DESCRIPTOR));
    }

    #[test]
    fn state_name_into_descriptor_invalid_data_scope() {
        let opaque_value = 0x0D83063EA3BE51F5; // this is `SAMPLE_STATE_NAME` with data scope set to 0x06
        let result: Result<WnfStateNameDescriptor, _> = WnfStateName::from_opaque_value(opaque_value).try_into();

        assert_eq!(
            result,
            Err(WnfStateNameDescriptorFromStateNameError::InvalidDataScope(0x06))
        );
    }

    #[test]
    fn descriptor_into_state_name_success() {
        let result: Result<WnfStateName, _> = SAMPLE_DESCRIPTOR.try_into();

        assert_eq!(result, Ok(SAMPLE_STATE_NAME));
    }

    #[test]
    fn descriptor_into_state_name_invalid_version() {
        let descriptor = WnfStateNameDescriptor {
            version: 1 << 4,
            ..SAMPLE_DESCRIPTOR
        };
        let result: Result<WnfStateName, _> = descriptor.try_into();

        assert_eq!(result, Err(WnfStateNameFromDescriptorError::InvalidVersion(1 << 4)));
    }

    #[test]
    fn descriptor_into_state_name_invalid_unique_id() {
        let descriptor = WnfStateNameDescriptor {
            unique_id: 1 << 21,
            ..SAMPLE_DESCRIPTOR
        };
        let result: Result<WnfStateName, _> = descriptor.try_into();

        assert_eq!(result, Err(WnfStateNameFromDescriptorError::InvalidUniqueId(1 << 21)));
    }
}
