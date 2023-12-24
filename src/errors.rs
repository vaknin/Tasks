use std::net::AddrParseError;

use thiserror::Error;

// Define your custom error type
#[derive(Error, Debug)]
pub enum MyError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Decode error: {0}")]
    Decode(#[from] prost::DecodeError),

    #[error("Encode error: {0}")]
    Encode(#[from] prost::EncodeError),

    #[error("Address parse error: {0}")]
    AddrParse(#[from] AddrParseError),

    #[error("Unable to server: {0}")]
    Transport(#[from] tonic::transport::Error),
}

// Implement conversion from MyError to tonic::Status
impl From<MyError> for tonic::Status {
    fn from(e: MyError) -> Self {
        tonic::Status::internal(e.to_string())
    }
}
