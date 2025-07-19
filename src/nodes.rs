use serde::{Deserialize, Serialize};

/// Comparison operators
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ComparisonOp {
    GreaterThan,
    LessThan,
    Equal,
}

/// Single enum for all node operations
/// 
/// To add a new node:
/// 1. Add variant here
/// 2. Add compute logic in engine.rs
/// 3. That's it! No YAML parsing changes needed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum NodeOp {
    /// Constant value node
    Constant { value: f64 },
    
    /// Input node that receives streaming data
    Input { input_index: usize },
    
    /// Add two values: a + b
    Add { inputs: Vec<usize> },
    
    /// Multiply two values: a * b  
    Multiply { inputs: Vec<usize> },
    
    /// Sum multiple values
    Sum { inputs: Vec<usize> },
    
    /// Multiply by constant: input * factor
    ConstantProduct { inputs: Vec<usize>, factor: f64 },
    
    /// Comparison operations
    Comparison { inputs: Vec<usize>, op: ComparisonOp },
    
    /// Power: a^b
    Pow { inputs: Vec<usize> },
}

impl NodeOp {
    /// Get inputs that this node depends on
    pub fn inputs(&self) -> Vec<usize> {
        match self {
            NodeOp::Constant { .. } => vec![],
            NodeOp::Input { .. } => vec![],
            NodeOp::Add { inputs } |
            NodeOp::Multiply { inputs } |
            NodeOp::Sum { inputs } |
            NodeOp::ConstantProduct { inputs, .. } |
            NodeOp::Comparison { inputs, .. } |
            NodeOp::Pow { inputs } => inputs.clone(),
        }
    }
    
    /// Update input indices (used during YAML parsing to resolve string refs)
    pub fn update_inputs(&mut self, new_inputs: Vec<usize>) {
        match self {
            NodeOp::Constant { .. } => {},
            NodeOp::Input { .. } => {},
            NodeOp::Add { inputs } |
            NodeOp::Multiply { inputs } |
            NodeOp::Sum { inputs } |
            NodeOp::ConstantProduct { inputs, .. } |
            NodeOp::Comparison { inputs, .. } |
            NodeOp::Pow { inputs } => *inputs = new_inputs,
        }
    }
}