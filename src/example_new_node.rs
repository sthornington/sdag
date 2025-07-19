// Example: Adding a new node type is DEAD SIMPLE!

use crate::{define_simple_node, EvalNode, NodeId};
use std::collections::HashMap;

// Step 1: Define the node with the macro
define_simple_node!(
    Max,
    tag = "max",
    fields = { children: Vec<NodeId> }
);

// Step 2: Implement the eval trait - that's it!
impl EvalNode for MaxNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        self.children
            .iter()
            .map(|&id| values[id])
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

// DONE! The macro automatically generates:
// - MaxNode struct
// - Max Python class with id and children fields  
// - Graph::max() method for Python
// - Automatic registration with the arena engine
// - Automatic registration with the Python module
// - YAML serialization/deserialization
// - All the py_node boilerplate

// Another example - Absolute value node:
define_simple_node!(
    Abs,
    tag = "abs",
    fields = { input: NodeId }
);

impl EvalNode for AbsNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        values[self.input].abs()
    }
}

// Power node:
define_simple_node!(
    Pow,
    tag = "pow",
    fields = { base: NodeId, exponent: f64 }
);

impl EvalNode for PowNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        values[self.base].powf(self.exponent)
    }
}

// That's it! In Python you can now do:
// g = Graph()
// x = g.input("x")
// y = g.input("y")
// max_xy = g.max([x, y])
// abs_x = g.abs(x)
// x_squared = g.pow(x, 2.0)