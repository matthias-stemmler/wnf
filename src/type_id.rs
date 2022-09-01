use std::fmt::{Debug, Display, Formatter};
use std::{fmt, io, ptr};

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub struct GUID(windows::core::GUID);

impl GUID {
    pub fn new() -> io::Result<Self> {
        Ok(Self(windows::core::GUID::new()?))
    }

    pub const fn zeroed() -> Self {
        Self(windows::core::GUID::zeroed())
    }

    pub const fn from_values(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self(windows::core::GUID::from_values(data1, data2, data3, data4))
    }

    pub const fn from_u128(uuid: u128) -> Self {
        Self(windows::core::GUID::from_u128(uuid))
    }

    pub const fn to_u128(&self) -> u128 {
        self.0.to_u128()
    }
}

impl Debug for GUID {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&str> for GUID {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<u128> for GUID {
    fn from(value: u128) -> Self {
        Self(value.into())
    }
}

impl From<GUID> for u128 {
    fn from(value: GUID) -> Self {
        value.to_u128()
    }
}

#[cfg(feature = "windows")]
impl From<windows::core::GUID> for GUID {
    fn from(guid: windows::core::GUID) -> Self {
        Self(guid)
    }
}

#[cfg(feature = "winapi")]
impl From<winapi::shared::guiddef::GUID> for GUID {
    fn from(guid: winapi::shared::guiddef::GUID) -> Self {
        Self::from_values(guid.Data1, guid.Data2, guid.Data3, guid.Data4)
    }
}

#[cfg(feature = "uuid")]
impl From<uuid::Uuid> for GUID {
    fn from(uuid: uuid::Uuid) -> Self {
        uuid.as_u128().into()
    }
}

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub(crate) struct TypeId(Option<windows::core::GUID>);

impl TypeId {
    pub(crate) fn none() -> Self {
        Self(None)
    }

    pub(crate) fn as_ptr(&self) -> *const windows::core::GUID {
        match self.0 {
            Some(guid) => &guid,
            None => ptr::null(),
        }
    }
}

impl From<GUID> for TypeId {
    fn from(guid: GUID) -> Self {
        Self(Some(guid.0))
    }
}

impl Debug for TypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Display for TypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
