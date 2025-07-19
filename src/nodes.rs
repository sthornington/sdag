use serde::{Deserialize, Serialize};

/// Single enum for all node operations - enables static dispatch for performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeOp {
    /// Constant value node
    Constant(f64),
    
    /// Input node that receives streaming data
    Input { input_index: usize },
    
    /// Add two values: a + b
    Add { a: usize, b: usize },
    
    /// Multiply two values: a * b
    Multiply { a: usize, b: usize },
    
    /// Sum multiple values
    Sum { inputs: Vec<usize> },
    
    /// Multiply by constant: input * factor
    ConstantProduct { input: usize, factor: f64 },
    
    /// Comparison operations
    Comparison { a: usize, b: usize, op: ComparisonOp },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ComparisonOp {
    GreaterThan,
    LessThan,
    Equal,
}