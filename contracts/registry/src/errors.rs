use core::fmt;
use xlm_ns_common::CommonError;

#[derive(Debug)]
pub enum RegistryError {
    AlreadyRegistered,
    NotFound,
    NotYetClaimable,
    NotActive,
    Unauthorized,
    MetadataTooLong,
    InvalidExpiry,
    InvalidGracePeriod,
    Locked,
    Validation(CommonError),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRegistered => f.write_str("name is already registered"),
            Self::NotFound => f.write_str("name was not found"),
            Self::NotYetClaimable => f.write_str("expired name is still in its grace period"),
            Self::NotActive => f.write_str("name is not currently active"),
            Self::Unauthorized => f.write_str("caller is not authorized for this name"),
            Self::MetadataTooLong => f.write_str("metadata uri exceeds the allowed length"),
            Self::InvalidExpiry => {
                f.write_str("expires_at must be greater than or equal to the current time")
            }
            Self::InvalidGracePeriod => {
                f.write_str("grace_period_ends_at must be greater than or equal to expires_at")
            }
            Self::Locked => f.write_str("name is locked for dispute resolution"),
            Self::Validation(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<CommonError> for RegistryError {
    fn from(value: CommonError) -> Self {
        Self::Validation(value)
    }
}
