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
            #[pyclass(name = stringify!($name))]
            pub struct $name {
                #[pyo3(get)]
                pub id: String,
                $(
                    #[pyo3(get)]
                    pub $field: define_simple_node!(@py_type $field_ty),
                )*
            }
            
            // 3. Graph builder method - automatically added
            impl crate::Graph {
                pub fn [<$name:snake>](&mut self, py: pyo3::Python, $($field: define_simple_node!(@py_type $field_ty)),*) -> pyo3::PyObject {
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
            
            // 4. Auto-register with Python module
            inventory::submit! {
                crate::NodeRegistration {
                    name: stringify!($name),
                    register: |m: &pyo3::types::PyModule| -> pyo3::PyResult<()> {
                        m.add_class::<$name>()?;
                        Ok(())
                    }
                }
            }
            
            // 5. Arena evaluation - users just implement EvalNode trait
            impl crate::ArenaEval for [<$name Node>] {
                fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
                    <Self as crate::EvalNode>::eval(self, values, inputs)
                }
            }
            
            // 6. Auto-register arena builder
            inventory::submit! {
                crate::ArenaNodeBuilder {
                    tag: $tag,
                    build: |node: &crate::engine::ArenaNode| -> Result<Box<dyn crate::ArenaEval>, String> {
                        // Extract fields from arena node
                        $(let $field = define_simple_node!(@extract_field node, stringify!($field), $field_ty)?;)*
                        
                        Ok(Box::new([<$name Node>] {
                            $($field,)*
                        }))
                    }
                }
            }
        }
    };
    
    // Type conversions
    (@py_type crate::NodeId) => { pyo3::PyObject };
    (@py_type Vec<crate::NodeId>) => { Vec<pyo3::PyObject> };
    (@py_type $t:ty) => { $t };
    
    // Field extraction from arena
    (@extract_field $node:expr, $field:expr, crate::NodeId) => {
        match $node.fields.get($field) {
            Some(crate::engine::FieldValue::One(id)) => Ok(*id),
            _ => Err(format!("Expected NodeId for field {}", $field)),
        }
    };
    (@extract_field $node:expr, $field:expr, Vec<crate::NodeId>) => {
        match $node.fields.get($field) {
            Some(crate::engine::FieldValue::Many(ids)) => Ok(ids.clone()),
            _ => Err(format!("Expected Vec<NodeId> for field {}", $field)),
        }
    };
    (@extract_field $node:expr, $field:expr, f64) => {
        match $node.fields.get($field) {
            Some(crate::engine::FieldValue::Float(f)) => Ok(*f),
            _ => Err(format!("Expected f64 for field {}", $field)),
        }
    };
    (@extract_field $node:expr, $field:expr, String) => {
        match $node.fields.get($field) {
            Some(crate::engine::FieldValue::Str(s)) => Ok(s.clone()),
            _ => Err(format!("Expected String for field {}", $field)),
        }
    };
}

// The ONLY trait users need to implement
pub trait EvalNode {
    fn eval(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64;
}

// Internal trait for arena evaluation
pub trait ArenaEval: Send + Sync {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64;
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