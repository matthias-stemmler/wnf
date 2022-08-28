use num_traits::FromPrimitive;
use std::fmt;
use std::fmt::{Display, Formatter};
use thiserror::Error;

const STATE_NAME_XOR_KEY: u64 = 0x41C64E6DA3BC0074;

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, Hash, PartialEq)]
#[repr(u8)]
pub enum WnfStateNameLifetime {
    /// Provisioned with the system, lives forever
    /// Persisted under HKLM\SYSTEM\CurrentControlSet\Control\Notifications
    WellKnown = 0, // -> HKLM\SYSTEM\CurrentControlSet\Control\Notifications

    /// Lives forever
    /// Persisted under HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Notifications
    Permanent = 1,

    /// Lives until system reboot ("volatile")
    /// Persisted under HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\VolatileNotifications
    Persistent = 2, // system uptime (aka "volatile") -> HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\VolatileNotifications

    /// Lives as long as the process that created it
    /// Not persisted
    Temporary = 3,
}

// Note: everything >4 works, some implementations define PhysicalMachine = 5, Process never seems to work
#[derive(Clone, Copy, Debug, Eq, FromPrimitive, Hash, PartialEq)]
#[repr(u8)]
pub enum WnfDataScope {
    System = 0,
    Session = 1,
    User = 2,
    Process = 3,
    Machine = 4,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WnfStateNameDescriptor {
    pub version: u8,
    pub lifetime: WnfStateNameLifetime,
    pub data_scope: WnfDataScope,
    pub is_permanent: bool,
    pub unique_id: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WnfStateName {
    opaque_value: u64,
}

impl WnfStateName {
    pub const fn from_opaque_value(opaque_value: u64) -> Self {
        Self { opaque_value }
    }

    pub const fn opaque_value(self) -> u64 {
        self.opaque_value
    }
}

impl TryFrom<WnfStateNameDescriptor> for WnfStateName {
    type Error = WnfStateNameFromDescriptorError;

    fn try_from(descriptor: WnfStateNameDescriptor) -> Result<Self, Self::Error> {
        if descriptor.version >= (1 << 4) {
            return Err(WnfStateNameFromDescriptorError::InvalidVersion(descriptor.version));
        }

        if descriptor.unique_id >= (1 << 53) {
            return Err(WnfStateNameFromDescriptorError::InvalidUniqueId(descriptor.unique_id));
        }

        let transparent_value = descriptor.version as u64
            + ((descriptor.lifetime as u64) << 4)
            + ((descriptor.data_scope as u64) << 6)
            + ((descriptor.is_permanent as u64) << 10)
            + (descriptor.unique_id << 11);

        let opaque_value = transparent_value ^ STATE_NAME_XOR_KEY;

        Ok(Self { opaque_value })
    }
}

impl TryFrom<WnfStateName> for WnfStateNameDescriptor {
    type Error = WnfStateNameDescriptorFromStateNameError;

    fn try_from(state_name: WnfStateName) -> Result<Self, Self::Error> {
        let transparent_value = state_name.opaque_value ^ STATE_NAME_XOR_KEY;

        let lifetime_value = ((transparent_value >> 4) & 0x3) as u8;
        let data_scope_value = ((transparent_value >> 6) & 0xf) as u8;

        Ok(Self {
            version: (transparent_value & 0xf) as u8,
            lifetime: WnfStateNameLifetime::from_u8(lifetime_value).ok_or(
                WnfStateNameDescriptorFromStateNameError::InvalidLifetime(lifetime_value),
            )?,
            data_scope: WnfDataScope::from_u8(data_scope_value).ok_or(
                WnfStateNameDescriptorFromStateNameError::InvalidDataScope(data_scope_value),
            )?,
            is_permanent: transparent_value & (1 << 10) != 0,
            unique_id: transparent_value >> 11,
        })
    }
}

impl Display for WnfStateName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:#010x}", self.opaque_value)
    }
}

#[derive(Debug, Error)]
pub enum WnfStateNameFromDescriptorError {
    #[error("invalid version: {0}")]
    InvalidVersion(u8),

    #[error("invalid unique id: {0}")]
    InvalidUniqueId(u64),
}

#[derive(Debug, Error)]
pub enum WnfStateNameDescriptorFromStateNameError {
    #[error("invalid lifetime: {0}")]
    InvalidLifetime(u8),

    #[error("invalid data scope: {0}")]
    InvalidDataScope(u8),
}
