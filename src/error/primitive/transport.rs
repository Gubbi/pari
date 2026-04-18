//! Transport and actor-boundary primitive errors.

use pari_macros::primitive_with_fields;

/// A request could not be sent across the intended boundary.
#[primitive_with_fields]
pub struct RequestTransportUnavailable {
    operation: String,
    boundary: String,
}

/// A request channel rejected or lost an outbound request.
#[primitive_with_fields]
pub struct RequestChannelSendFailed {
    operation: String,
    boundary: String,
}

/// A reply was never delivered on the expected response channel.
#[primitive_with_fields]
pub struct ReplyChannelDropped {
    operation: String,
    boundary: String,
}

/// The target actor terminated before it could complete the request.
#[primitive_with_fields]
pub struct ActorTerminatedMidRequest {
    operation: String,
    actor: String,
}

/// A request payload reaching the boundary did not match the expected operation contract.
#[primitive_with_fields]
pub struct MalformedRequestPayload {
    operation: String,
    reason: String,
}

/// A response shape did not match the request/response protocol for the operation.
#[primitive_with_fields]
pub struct RequestResponseProtocolMismatch {
    operation: String,
    expected: String,
    actual: String,
}

/// Internal actor state could not support dispatch or the requested transition.
#[primitive_with_fields]
pub struct ActorStateTransitionInvariantViolation {
    operation: String,
    reason: String,
}
