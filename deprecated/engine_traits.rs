use std::collections::HashMap;
use crate::arena::{NodeId, ArenaNode};

/// Base evaluation trait for nodes
pub trait EvalNode: Send + Sync {
    /// Evaluate using input row (for non-arena evaluation)
    fn eval_row(&self, row: &HashMap<String, f64>) -> f64;
    
    /// Evaluate using computed values array (for arena evaluation)
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64;
}

/// Engine trait for different evaluation strategies
pub trait Engine: Send + Sync {
    /// Name of the engine
    fn name(&self) -> &str;
    
    /// Evaluate a graph with given input rows
    fn evaluate(
        &self,
        nodes: &[Box<dyn EvalNode>],
        root: NodeId,
        outputs: &[NodeId],
        rows: Vec<HashMap<String, f64>>,
    ) -> Vec<HashMap<String, f64>>;
}

/// Node builder from arena representation
pub trait NodeBuilder: Send + Sync {
    /// Build a node from arena representation
    fn build(&self, node: &ArenaNode) -> Result<Box<dyn EvalNode>, String>;
}

/// Registry for node builders
pub struct NodeRegistry {
    builders: HashMap<String, Box<dyn NodeBuilder>>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            builders: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, node_type: &str, builder: Box<dyn NodeBuilder>) {
        self.builders.insert(node_type.to_string(), builder);
    }
    
    pub fn build(&self, node: &ArenaNode) -> Result<Box<dyn EvalNode>, String> {
        self.builders
            .get(&node.node_type)
            .ok_or_else(|| format!("Unknown node type: {}", node.node_type))?
            .build(node)
    }
}