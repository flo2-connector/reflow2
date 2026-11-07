//! Error types for DynoGraph.

use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DynoError {
    #[error("Schema error: {0}")]
    Schema(String),

    #[error("Validation error on {node_type}.{property}: {message}")]
    Validation {
        node_type: String,
        property: String,
        message: String,
    },

    #[error("Validation error on edge {edge_type}.{property}: {message}")]
    EdgeValidation {
        edge_type: String,
        property: String,
        message: String,
    },

    #[error("Node not found: {node_type} {node_id}")]
    NodeNotFound { node_type: String, node_id: String },

    #[error("Edge not found: {edge_type} from {from_id} to {to_id}")]
    EdgeNotFound {
        edge_type: String,
        from_id: String,
        to_id: String,
    },

    #[error("Invalid edge: {edge_type} cannot connect {from_type} to {to_type}")]
    InvalidEdge {
        edge_type: String,
        from_type: String,
        to_type: String,
    },

    #[error("Unknown node type: {0}")]
    UnknownNodeType(String),

    #[error("Unknown edge type: {0}")]
    UnknownEdgeType(String),

    #[error("Invalid {field}: {message}")]
    InvalidKeySegment { field: String, message: String },

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Resolution error: {0}")]
    Resolution(String),

    #[error("Extraction error: {0}")]
    Extraction(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    /// A long read was told to stop at a check point (`crate::stop`): its
    /// time budget ran out (`reason: budget`) or its caller gave up
    /// (`caller_gone`). Never a fault: whoever opened the scope says what was
    /// not computed.
    #[error("Stopped ({reason}) at {at}")]
    Stopped { reason: String, at: String },
}
