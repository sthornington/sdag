/// Simple macro for defining nodes - generates ALL boilerplate in one place
#[macro_export]
macro_rules! define_node {
    (
        $name:ident,
        tag = $tag:literal,
        fields = { $($field:ident : $field_ty:ty),* $(,)? },
        eval = $eval_body:expr
    ) => {
        paste::paste! {
            // 1. The node struct
            #[derive(Debug, Clone)]
            pub struct [<$name Node>] {
                $(pub $field: $field_ty,)*
            }
            
            // 2. EvalNode implementation with user's eval logic
            impl $crate::simple_node_macro::EvalNode for [<$name Node>] {
                fn eval(&self, values: &[f64], inputs: &std::collections::HashMap<String, f64>) -> f64 {
                    $eval_body
                }
            }
            
            // 3. ArenaEval implementation (automatic)
            impl $crate::simple_node_macro::ArenaEval for [<$name Node>] {
                fn eval_arena(&self, values: &[f64], inputs: &std::collections::HashMap<String, f64>) -> f64 {
                    self.eval(values, inputs)
                }
            }
            
            // 4. Python wrapper class
            #[pyclass(name = $tag)]
            pub struct $name {
                #[pyo3(get)]
                pub id: String,
                $(
                    #[pyo3(get)]
                    pub $field: define_node!(@py_type $field_ty),
                )*
            }
            
            // 5. Register the arena builder
            inventory::submit! {
                $crate::NodeBuilder {
                    tag: $tag,
                    build: |node: &$crate::engine::ArenaNode| -> Result<Box<dyn $crate::simple_node_macro::ArenaEval>, String> {
                        $(let $field = define_node!(@extract_field node, stringify!($field), $field_ty)?;)*
                        Ok(Box::new([<$name Node>] {
                            $($field,)*
                        }))
                    }
                }
            }
            
            // 6. Register for Python module
            inventory::submit! {
                $crate::PyNodeRegistration {
                    register: |m: &pyo3::types::PyModule| -> pyo3::PyResult<()> {
                        m.add_class::<$name>()?;
                        Ok(())
                    }
                }
            }
        }
    };
    
    // Helper rules for type conversion
    (@py_type NodeId) => { pyo3::PyObject };
    (@py_type Vec<NodeId>) => { Vec<pyo3::PyObject> };
    (@py_type $t:ty) => { $t };
    
    // Helper rules for field extraction
    (@extract_field $node:expr, $field:expr, NodeId) => {
        match $node.fields.get($field) {
            Some($crate::engine::FieldValue::One(id)) => Ok(*id),
            _ => Err(format!("Expected NodeId for field {}", $field)),
        }
    };
    (@extract_field $node:expr, $field:expr, Vec<NodeId>) => {
        match $node.fields.get($field) {
            Some($crate::engine::FieldValue::Many(ids)) => Ok(ids.clone()),
            _ => Err(format!("Expected Vec<NodeId> for field {}", $field)),
        }
    };
    (@extract_field $node:expr, $field:expr, f64) => {
        match $node.fields.get($field) {
            Some($crate::engine::FieldValue::Float(f)) => Ok(*f),
            _ => Err(format!("Expected f64 for field {}", $field)),
        }
    };
    (@extract_field $node:expr, $field:expr, String) => {
        match $node.fields.get($field) {
            Some($crate::engine::FieldValue::Str(s)) => Ok(s.clone()),
            _ => Err(format!("Expected String for field {}", $field)),
        }
    };
}

// Registration types
pub struct NodeBuilder {
    pub tag: &'static str,
    pub build: fn(&crate::engine::ArenaNode) -> Result<Box<dyn crate::simple_node_macro::ArenaEval>, String>,
}

inventory::collect!(NodeBuilder);

pub struct PyNodeRegistration {
    pub register: fn(&pyo3::types::PyModule) -> pyo3::PyResult<()>,
}

inventory::collect!(PyNodeRegistration);

// Helper to build nodes from arena
pub fn build_node_from_arena(node: &crate::engine::ArenaNode) -> Result<Box<dyn crate::simple_node_macro::ArenaEval>, String> {
    for builder in inventory::iter::<NodeBuilder> {
        if builder.tag == node.tag {
            return (builder.build)(node);
        }
    }
    Err(format!("Unknown node type: {}", node.tag))
}

// Helper to register all nodes with Python
pub fn register_all_py_nodes(m: &pyo3::types::PyModule) -> pyo3::PyResult<()> {
    for reg in inventory::iter::<PyNodeRegistration> {
        (reg.register)(m)?;
    }
    Ok(())
}