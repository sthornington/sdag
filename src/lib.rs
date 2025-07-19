use thiserror::Error;

pub mod node;
pub mod dag;
pub mod yaml;
#[cfg(feature = "python")]
pub mod python;

pub use node::{Node, NodeFactory, NodeRegistry, Value};
pub use dag::{Dag, DagBuilder};
pub use yaml::{DagYaml, NodeYaml};

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