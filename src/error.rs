use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecurityCreateError {
    #[error("failed to create security descriptor: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error)]
pub enum WnfCreateError {
    #[error("failed to create WNF state name: security error {0}")]
    Security(#[from] SecurityCreateError),

    #[error("failed to create WNF state name: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error)]
pub enum WnfInfoError {
    #[error("failed to determine WNF state info: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error)]
pub enum WnfDeleteError {
    #[error("failed to delete WNF state name: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error)]
pub enum WnfQueryError {
    #[error("failed to query WNF state data: data has wrong size (expected {expected}, got {actual})")]
    WrongSize { expected: usize, actual: usize },

    #[error(
        "failed to query WNF state data: data has wrong size (expected multiple of {expected_modulus}, got {actual})"
    )]
    WrongSizeMultiple { expected_modulus: usize, actual: usize },

    #[error("failed to query WNF state data: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error)]
pub enum WnfUpdateError {
    #[error("failed to update WNF state data: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error)]
pub enum WnfApplyError {
    #[error("failed to apply operation to WNF state data: failed to query data {0}")]
    Query(#[from] WnfQueryError),

    #[error("failed to apply operation to WNF state data: failed to update data {0}")]
    Update(#[from] WnfUpdateError),
}

#[derive(Debug, Error)]
pub enum WnfSubscribeError {
    #[error("failed to subscribe to WNF state change: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}

#[derive(Debug, Error)]
pub enum WnfUnsubscribeError {
    #[error("failed to unsubscribe from WNF state change: Windows error code {:#010x}", .0.code().0)]
    Windows(#[from] windows::core::Error),
}
