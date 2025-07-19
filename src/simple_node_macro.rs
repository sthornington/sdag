/// Dead simple macro for defining nodes - just specify fields and implement eval
#[macro_export]
macro_rules! define_simple_node {
    (
        $name:ident,
        tag = $tag:literal,
        fields = { $($field:ident : $field_ty:ty),* $(,)? }
    ) => {
        paste::paste! {
            // 1. The node struct that users implement
            #[derive(Debug, Clone)]
            pub struct [<$name Node>] {
                $(pub $field: $field_ty,)*
            }
            
            impl [<$name Node>] {
                pub const TYPE: &'static str = $tag;
            }
            
            // 2. Python wrapper - automatically generated
            #[py_node([<$name Node>]::TYPE, $($field),*)]
            #[pyclass]
            pub struct $name {
                #[pyo3(get)]
                pub id: String,
                $(
                    #[pyo3(get)]
                    pub $field: define_simple_node!(@py_type $field_ty),
                )*
            }
            
            // 3. Graph builder method - automatically added
            // Use a custom method name based on the tag to avoid keyword conflicts
            impl crate::Graph {
                paste::paste! {
                    pub fn [< create_ $tag:snake >](&mut self, py: pyo3::Python, $($field: define_simple_node!(@py_type $field_ty)),*) -> pyo3::PyObject {
                        use pyo3::IntoPy;
                        let id = format!("n{}", self.counter);
                        self.counter += 1;
                        let node = $name { 
                            id: id.clone(), 
                            $($field),* 
                        };
                        let py_node = node.into_py(py);
                        self.registry.insert(id, py_node.clone());
                        py_node
                    }
                }
            }
            
            // 4. Auto-register with Python module
            inventory::submit! {
                crate::simple_node_macro::NodeRegistration {
                    name: $tag,
                    register: |m: &pyo3::types::PyModule| -> pyo3::PyResult<()> {
                        m.add_class::<$name>()?;
                        Ok(())
                    }
                }
            }
            
            // 5. Arena evaluation - users just implement EvalNode trait
            impl crate::simple_node_macro::ArenaEval for [<$name Node>] {
                fn eval_arena(&self, values: &[f64], inputs: &std::collections::HashMap<String, f64>) -> f64 {
                    <Self as crate::simple_node_macro::EvalNode>::eval(self, values, inputs)
                }
            }
            
            // 6. Auto-register arena builder
            inventory::submit! {
                crate::simple_node_macro::ArenaNodeBuilder {
                    tag: $tag,
                    build: |node: &crate::engine::ArenaNode| -> Result<Box<dyn crate::simple_node_macro::ArenaEval>, String> {
                        // Extract fields from arena node using helper function
                        $(let $field = crate::simple_node_macro::extract_field(node, stringify!($field), stringify!($field_ty))?;)*
                        
                        Ok(Box::new([<$name Node>] {
                            $($field,)*
                        }))
                    }
                }
            }
        }
    };
    
    // Type conversions
    (@py_type NodeId) => { pyo3::PyObject };
    (@py_type Vec<NodeId>) => { Vec<pyo3::PyObject> };
    (@py_type $t:ty) => { $t };
}

// Helper function to extract fields based on string type
pub fn extract_field<T>(node: &crate::engine::ArenaNode, field_name: &str, field_type: &str) -> Result<T, String> 
where T: ExtractField {
    T::extract(node, field_name, field_type)
}

// Trait for extractable field types
pub trait ExtractField: Sized {
    fn extract(node: &crate::engine::ArenaNode, field_name: &str, field_type: &str) -> Result<Self, String>;
}

use crate::engine::NodeId;

impl ExtractField for NodeId {
    fn extract(node: &crate::engine::ArenaNode, field_name: &str, _field_type: &str) -> Result<Self, String> {
        match node.fields.get(field_name) {
            Some(crate::engine::FieldValue::One(id)) => Ok(*id),
            _ => Err(format!("Expected NodeId for field {}", field_name)),
        }
    }
}

impl ExtractField for Vec<NodeId> {
    fn extract(node: &crate::engine::ArenaNode, field_name: &str, _field_type: &str) -> Result<Self, String> {
        match node.fields.get(field_name) {
            Some(crate::engine::FieldValue::Many(ids)) => Ok(ids.clone()),
            _ => Err(format!("Expected Vec<NodeId> for field {}", field_name)),
        }
    }
}

impl ExtractField for f64 {
    fn extract(node: &crate::engine::ArenaNode, field_name: &str, _field_type: &str) -> Result<Self, String> {
        match node.fields.get(field_name) {
            Some(crate::engine::FieldValue::Float(f)) => Ok(*f),
            _ => Err(format!("Expected f64 for field {}", field_name)),
        }
    }
}

impl ExtractField for String {
    fn extract(node: &crate::engine::ArenaNode, field_name: &str, _field_type: &str) -> Result<Self, String> {
        match node.fields.get(field_name) {
            Some(crate::engine::FieldValue::Str(s)) => Ok(s.clone()),
            _ => Err(format!("Expected String for field {}", field_name)),
        }
    }
}

// The ONLY trait users need to implement
pub trait EvalNode {
    fn eval(&self, values: &[f64], inputs: &std::collections::HashMap<String, f64>) -> f64;
}

// Internal trait for arena evaluation
pub trait ArenaEval: Send + Sync {
    fn eval_arena(&self, values: &[f64], inputs: &std::collections::HashMap<String, f64>) -> f64;
}

// Node registration for Python
pub struct NodeRegistration {
    pub name: &'static str,
    pub register: fn(&pyo3::types::PyModule) -> pyo3::PyResult<()>,
}

inventory::collect!(NodeRegistration);

// Arena node builder registration
pub struct ArenaNodeBuilder {
    pub tag: &'static str,
    pub build: fn(&crate::engine::ArenaNode) -> Result<Box<dyn ArenaEval>, String>,
}

inventory::collect!(ArenaNodeBuilder);

// Helper to register all nodes with Python
pub fn register_all_nodes(m: &pyo3::types::PyModule) -> pyo3::PyResult<()> {
    for registration in inventory::iter::<NodeRegistration> {
        (registration.register)(m)?;
    }
    Ok(())
}

// Helper to build arena nodes
pub fn build_arena_node(node: &crate::engine::ArenaNode) -> Result<Box<dyn ArenaEval>, String> {
    for builder in inventory::iter::<ArenaNodeBuilder> {
        if builder.tag == node.tag {
            return (builder.build)(node);
        }
    }
    Err(format!("Unknown node type: {}", node.tag))
}