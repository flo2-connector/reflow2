//! Schema, value and error vocabulary — the types every other module speaks.
//!
//! Absorbed verbatim from `dynograph-core` at v0.12.0; see [`super`] for the
//! full provenance block and for why this module is public where the other
//! absorbed ones are not.

mod error;
mod schema;
mod value;

pub use error::DynoError;
pub use schema::{
    ConfusedWith, Discrimination, EdgeEndpoint, EdgeReading, EdgeTypeDef, ExtractionInclude,
    MODIFIER_KINDS, NodeTypeDef, PropertyDef, PropertyType, READING_BASES, READING_FORMS,
    RELATION_PRIMITIVES, Reading, ReadingSplit, ResolutionConfig, ResolutionStrategy, Schema,
    TwinEnd, TwinOf,
};
pub use value::Value;
