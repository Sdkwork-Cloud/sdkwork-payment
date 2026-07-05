use sdkwork_contract_service::CommerceServiceError;

use crate::adapter::PaymentAdapterOperation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    InvalidRequest {
        operation: PaymentAdapterOperation,
        message: String,
    },
    InvalidResponse {
        operation: PaymentAdapterOperation,
        message: String,
    },
    UnsupportedCapability {
        provider_code: String,
        operation: PaymentAdapterOperation,
    },
    ProviderUnavailable {
        provider_code: String,
        message: String,
    },
    Transport {
        provider_code: String,
        message: String,
    },
}

impl ProviderError {
    pub fn invalid_request(operation: PaymentAdapterOperation, message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            operation,
            message: message.into(),
        }
    }

    pub fn invalid_response(operation: PaymentAdapterOperation, message: impl Into<String>) -> Self {
        Self::InvalidResponse {
            operation,
            message: message.into(),
        }
    }

    pub fn transport(provider_code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Transport {
            provider_code: provider_code.into(),
            message: message.into(),
        }
    }
}

impl From<ProviderError> for CommerceServiceError {
    fn from(error: ProviderError) -> Self {
        match error {
            ProviderError::InvalidRequest { message, .. }
            | ProviderError::InvalidResponse { message, .. } => Self::validation(message),
            ProviderError::UnsupportedCapability {
                provider_code,
                operation,
            } => Self::provider_unavailable(format!(
                "provider {provider_code} does not support {operation:?}"
            )),
            ProviderError::ProviderUnavailable { message, .. } => Self::provider_unavailable(message),
            ProviderError::Transport { message, .. } => Self::provider_unavailable(message),
        }
    }
}

pub type ProviderResult<T> = Result<T, ProviderError>;
