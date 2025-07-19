use crate::engine::{ArenaEvalNode, NodeId};
use std::collections::HashMap;

// Define all nodes using our macro

define_node! {
    name: Input,
    type_tag: "input",
    fields: {
        name: String,
    },
    eval: |self, _values: &[f64]| {
        // Input nodes are handled specially in arena evaluation
        0.0
    }
}

define_node! {
    name: Const,
    type_tag: "const",
    fields: {
        value: f64,
    },
    eval: |self, _values: &[f64]| {
        self.value
    }
}

define_node! {
    name: Add,
    type_tag: "add",
    fields: {
        children: Vec<NodeId>,
    },
    eval: |self, values: &[f64]| {
        self.children.iter().map(|&id| values[id]).sum()
    }
}

define_node! {
    name: Mul,
    type_tag: "mul",
    fields: {
        children: Vec<NodeId>,
    },
    eval: |self, values: &[f64]| {
        self.children.iter().map(|&id| values[id]).product()
    }
}

define_node! {
    name: Div,
    type_tag: "div",
    fields: {
        left: NodeId,
        right: NodeId,
    },
    eval: |self, values: &[f64]| {
        let l = values[self.left];
        let r = values[self.right];
        if r == 0.0 {
            f64::NAN
        } else {
            l / r
        }
    }
}