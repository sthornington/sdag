use thiserror::Error;

pub mod engine;
pub mod nodes;
pub mod yaml;
#[cfg(feature = "python")]
pub mod python;

pub use engine::{Engine, NodeId};
pub use nodes::{NodeOp, ComparisonOp};
pub use yaml::DagYaml;

#[derive(Error, Debug)]
pub enum DagError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    
    #[error("Circular dependency detected")]
    CircularDependency,
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Node execution error: {0}")]
    ExecutionError(String),
    
    #[error("Type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String },
}

#[cfg(test)]
mod tests;