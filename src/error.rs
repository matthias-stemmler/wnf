use std::convert::Infallible;
use thiserror::Error;
use windows::Win32::Foundation::NTSTATUS;

#[derive(Debug, Error, PartialEq)]
pub enum SecurityCreateError {
    #[error("failed to create security descriptor: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfCreateError {
    #[error("failed to create WNF state name: security error {0}")]
    Security(#[from] SecurityCreateError),

    #[error("failed to create WNF state name: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfInfoError {
    #[error("failed to query WNF state information: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfDeleteError {
    #[error("failed to delete WNF state name: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfQueryError {
    #[error("failed to query WNF state data: data has wrong size (expected {expected}, got {actual})")]
    WrongSize { expected: usize, actual: usize },

    #[error(
        "failed to query WNF state data: data has wrong size (expected multiple of {expected_modulus}, got {actual})"
    )]
    WrongSizeMultiple { expected_modulus: usize, actual: usize },

    #[error("failed to query WNF state data: data has invalid bit pattern")]
    InvalidBitPattern,

    #[error("failed to query WNF state data: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfUpdateError {
    #[error("failed to update WNF state data: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error, PartialEq)]
#[error(transparent)]
pub struct WnfTransformError<E>(#[from] pub E);

#[derive(Debug, Error, PartialEq)]
pub enum WnfApplyError<E = Infallible> {
    #[error("failed to apply transformation to WNF state data: failed to query data: {0}")]
    Query(#[from] WnfQueryError),

    #[error("failed to apply transformation to WNF state data: failed to transform data: {0}")]
    Transform(#[from] WnfTransformError<E>),

    #[error("failed to apply transformation to WNF state data: failed to update data: {0}")]
    Update(#[from] WnfUpdateError),
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfSubscribeError {
    #[error("failed to subscribe to WNF state change: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error, PartialEq)]
pub enum WnfUnsubscribeError {
    #[error("failed to unsubscribe from WNF state change: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

macro_rules! impl_from_ntstatus {
    ($($t:ty,)*) => {
        $(
            impl From<NTSTATUS> for $t {
                fn from(result: NTSTATUS) -> Self {
                    let err: windows::core::Error = result.into();
                    err.into()
                }
            }
        )*
    }
}

impl_from_ntstatus![
    SecurityCreateError,
    WnfCreateError,
    WnfInfoError,
    WnfDeleteError,
    WnfQueryError,
    WnfUpdateError,
    WnfSubscribeError,
];
