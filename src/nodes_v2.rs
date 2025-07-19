use crate::arena::NodeId;
use crate::define_dag_node;

// Input node - reads from input row
define_dag_node! {
    InputNode {
        type_tag: "input",
        fields: {
            name: String,
        },
        eval_row: |self, row| {
            *row.get(&self.name).unwrap_or(&0.0)
        },
        eval_arena: |self, _values, inputs| {
            *inputs.get(&self.name).unwrap_or(&0.0)
        }
    }
}

// Constant node
define_dag_node! {
    ConstNode {
        type_tag: "const",
        fields: {
            value: f64,
        },
        eval_row: |self, _row| {
            self.value
        },
        eval_arena: |self, _values, _inputs| {
            self.value
        }
    }
}

// Addition node
define_dag_node! {
    AddNode {
        type_tag: "add",
        fields: {
            children: Vec<NodeId>,
        },
        eval_row: |_self, _row| {
            // Not used in arena mode
            0.0
        },
        eval_arena: |self, values, _inputs| {
            self.children.iter().map(|&id| values[id]).sum()
        }
    }
}

// Multiplication node
define_dag_node! {
    MulNode {
        type_tag: "mul",
        fields: {
            children: Vec<NodeId>,
        },
        eval_row: |_self, _row| {
            // Not used in arena mode
            0.0
        },
        eval_arena: |self, values, _inputs| {
            self.children.iter().map(|&id| values[id]).product()
        }
    }
}

// Division node
define_dag_node! {
    DivNode {
        type_tag: "div",
        fields: {
            left: NodeId,
            right: NodeId,
        },
        eval_row: |_self, _row| {
            // Not used in arena mode
            0.0
        },
        eval_arena: |self, values, _inputs| {
            let l = values[self.left];
            let r = values[self.right];
            if r == 0.0 {
                f64::NAN
            } else {
                l / r
            }
        }
    }
}

// Register all nodes
pub fn register_all_nodes(registry: &mut crate::engine_traits::NodeRegistry) {
    register_input_node(registry);
    register_const_node(registry);
    register_add_node(registry);
    register_mul_node(registry);
    register_div_node(registry);
}

// Register Python classes
pub fn register_all_python(m: &pyo3::types::PyModule) -> pyo3::PyResult<()> {
    register_input_node_py(m)?;
    register_const_node_py(m)?;
    register_add_node_py(m)?;
    register_mul_node_py(m)?;
    register_div_node_py(m)?;
    Ok(())
}