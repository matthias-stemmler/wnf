//! Dealing with type IDs of states

#![deny(unsafe_code)]

use std::fmt::{Debug, Display, Formatter};
use std::{fmt, io, ptr};

/// A Globally Unique Identifier (GUID)
///
/// This is used to (optionally) specify type IDs for states.
/// It provides [`From<T>`] implementations for various types `T` from foreign crates that also represent GUIDs.
/// These implementations are available when the respective Cargo features are enabled:
/// - Feature `windows`: [`From<windows::core::GUID>`](windows::core::GUID)
/// - Feature `winapi`: [`From<winapi::shared::guiddef::GUID>`](winapi::shared::guiddef::GUID)
/// - Feature `uuid`: [`From<uuid::Uuid>`](uuid::Uuid)
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GUID(windows::core::GUID);

impl GUID {
    /// Creates a unique GUID value
    ///
    /// # Errors
    /// Returns an error if creating the GUID fails
    pub fn new() -> io::Result<Self> {
        Ok(Self(windows::core::GUID::new()?))
    }

    /// Creates a GUID represented by the all-zero byte pattern
    pub const fn zeroed() -> Self {
        Self(windows::core::GUID::zeroed())
    }

    /// Creates a GUID with the given constant values
    pub const fn from_values(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self(windows::core::GUID::from_values(data1, data2, data3, data4))
    }

    /// Creates a GUID from a [`u128`] value
    pub const fn from_u128(uuid: u128) -> Self {
        Self(windows::core::GUID::from_u128(uuid))
    }

    /// Converts a GUID to a [`u128`] value
    pub const fn to_u128(&self) -> u128 {
        self.0.to_u128()
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
mod impl_windows {
    use super::*;

    impl From<GUID> for windows::core::GUID {
        fn from(guid: GUID) -> Self {
            guid.0
        }
    }

    impl From<windows::core::GUID> for GUID {
        fn from(guid: windows::core::GUID) -> Self {
            Self(guid)
        }
    }
}

#[cfg(feature = "winapi")]
mod impl_winapi {
    use super::*;

    impl From<GUID> for winapi::shared::guiddef::GUID {
        fn from(guid: GUID) -> Self {
            Self {
                Data1: guid.0.data1,
                Data2: guid.0.data2,
                Data3: guid.0.data3,
                Data4: guid.0.data4,
            }
        }
    }

    impl From<winapi::shared::guiddef::GUID> for GUID {
        fn from(guid: winapi::shared::guiddef::GUID) -> Self {
            Self::from_values(guid.Data1, guid.Data2, guid.Data3, guid.Data4)
        }
    }
}

#[cfg(feature = "uuid")]
mod impl_uuid {
    use super::*;

    impl From<GUID> for uuid::Uuid {
        fn from(guid: GUID) -> Self {
            Self::from_u128(guid.into())
        }
    }

    impl From<uuid::Uuid> for GUID {
        fn from(uuid: uuid::Uuid) -> Self {
            uuid.as_u128().into()
        }
    }
}

/// Internal helper type wrapping an optional GUID for use as a type ID of a state
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) struct TypeId(Option<windows::core::GUID>);

impl TypeId {
    /// Creates a [`TypeId`] containing no GUID for use in an untyped state
    pub(crate) const fn none() -> Self {
        Self(None)
    }

    /// Creates a [`TypeId`] containing the given [`GUID`]
    pub(crate) const fn from_guid(guid: GUID) -> Self {
        Self(Some(guid.0))
    }

    /// Returns a raw pointer to the underlying GUID, or a null pointer if there is none
    ///
    /// It is guaranteed that the returned pointer is either a null pointer or points to a valid GUID as long the
    /// instance of [`TypeId`] is live. The returned pointer can be passed to WNF APIs expecting an optional type id.
    pub(crate) const fn as_ptr(&self) -> *const windows::core::GUID {
        match self.0.as_ref() {
            Some(guid) => guid,
            None => ptr::null(),
        }
    }
}

impl From<GUID> for TypeId {
    fn from(guid: GUID) -> Self {
        Self(Some(guid.0))
    }
}

impl Display for TypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    #![allow(unsafe_code)]

    use super::*;

    #[test]
    fn type_id_from_guid_as_ptr() {
        let guid: GUID = GUID::from_u128(0x0011_2233_4455_6677_8899_AABB_CCDD_EEFF);

        let type_id: TypeId = guid.into();
        let ptr = type_id.as_ptr();

        assert!(!ptr.is_null());

        // SAFETY:
        // `ptr` points to a valid GUID by the safety conditions of `TypeId::as_ptr` because `!ptr.is_null()` and
        // `type_id` is live
        let windows_guid = unsafe { *ptr };

        assert_eq!(windows_guid.to_u128(), guid.to_u128());
    }

    #[test]
    fn type_id_none_as_ptr() {
        let type_id = TypeId::none();
        let ptr = type_id.as_ptr();

        assert!(ptr.is_null());
    }
}
