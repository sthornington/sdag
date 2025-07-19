/// Comprehensive macro that generates ALL boilerplate for a node type
#[macro_export]
macro_rules! define_node {
    (
        $node_name:ident {
            type_tag: $tag:literal,
            fields: { $($field:ident : $field_ty:ty),* $(,)? },
            eval: |$self:ident, $row:ident, $values:ident| $eval:expr
        }
    ) => {
        // Use paste for identifier manipulation
        paste::paste! {
            // 1. Engine-side node implementation
            #[derive(Debug, Clone)]
            pub struct [<$node_name Node>] {
                $(pub $field: $field_ty,)*
            }
            
            impl [<$node_name Node>] {
                pub const TYPE: &'static str = $tag;
            }
            
            // Implement the Node trait for row-based evaluation
            impl $crate::engine::Node for [<$node_name Node>] {
                fn eval(&self, $row: &std::collections::HashMap<String, f64>) -> f64 {
                    let $self = self;
                    let $values = &[];  // Not used in row mode
                    $eval
                }
            }
            
            // 2. YAML spec structure for deserialization
            #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            pub struct [<$node_name Spec>] {
                pub id: String,
                $(pub $field: define_node!(@spec_type $field_ty),)*
            }
            
            // 3. NodeDef implementation for building from YAML
            impl $crate::engine::NodeDef for [<$node_name Spec>] {
                const TYPE: &'static str = $tag;
                
                fn from_yaml(v: &serde_yaml::Value) -> Result<Box<dyn $crate::engine::Node + Send + Sync>, String> {
                    let spec: Self = serde_yaml::from_value(v.clone())
                        .map_err(|e| e.to_string())?;
                    
                    // Build node from spec
                    $(let $field = define_node!(@build_field spec.$field, $field_ty);)*
                    
                    let node = [<$node_name Node>] { $($field),* };
                    Ok(Box::new(node))
                }
                
                fn from_arena_spec(spec: &$crate::engine::ArenaNode) -> Result<Box<dyn $crate::engine::ArenaEvalNode>, String> {
                    $(let $field = define_node!(@extract_arena_field spec, stringify!($field), $field_ty)?;)*
                    
                    // Create arena evaluator wrapper
                    struct [<$node_name ArenaEval>] {
                        $(pub $field: $field_ty,)*
                    }
                    
                    impl $crate::engine::ArenaEvalNode for [<$node_name ArenaEval>] {
                        fn eval(&self, $values: &[f64]) -> f64 {
                            let $self = self;
                            let $row = &std::collections::HashMap::new();  // Not used in arena mode
                            $eval
                        }
                    }
                    
                    let node = [<$node_name ArenaEval>] { $($field),* };
                    Ok(Box::new(node))
                }
            }
            
            // 4. Register with inventory
            inventory::submit! {
                $crate::engine::Builder { 
                    tag: [<$node_name Spec>]::TYPE, 
                    build: [<$node_name Spec>]::from_yaml 
                }
            }
            
            // 5. Python wrapper class using py_node macro
            #[py_node([<$node_name Node>]::TYPE, $($field),*)]
            #[pyclass(name = stringify!($node_name))]
            pub struct $node_name {
                #[pyo3(get)]
                pub id: String,
                $(
                    #[pyo3(get)]
                    pub $field: define_node!(@py_field_type $field_ty),
                )*
            }
            
            // 6. Add to Python module registration
            impl $crate::PyModuleInit for $node_name {
                fn add_to_module(m: &pyo3::types::PyModule) -> pyo3::PyResult<()> {
                    m.add_class::<$node_name>()?;
                    Ok(())
                }
            }
            
            // 7. Graph builder method
            impl Graph {
                pub fn [<$node_name:snake>](&mut self, py: pyo3::Python, $($field: define_node!(@py_field_type $field_ty)),*) -> pyo3::PyObject {
                    let id = format!("n{}", self.counter);
                    self.counter += 1;
                    let node = $node_name { 
                        id: id.clone(), 
                        $($field),* 
                    };
                    let py_node = pyo3::IntoPy::into_py(node, py);
                    self.registry.insert(id, py_node.clone());
                    py_node
                }
            }
            
            // 8. Arena engine evaluation registration
            impl $crate::ArenaEngineRegistry {
                paste::paste! {
                    pub fn [<register_ $node_name:snake>]() {
                        $crate::ARENA_NODE_BUILDERS.write().unwrap().insert(
                            $tag,
                            Box::new(|node: &$crate::engine::ArenaNode, values: &[f64], inputs: &std::collections::HashMap<String, f64>| -> f64 {
                                $(let $field = define_node!(@extract_arena_field_runtime node, stringify!($field), $field_ty).unwrap();)*
                                let $self = &[<$node_name Node>] { $($field),* };
                                let $row = inputs;
                                let $values = values;
                                $eval
                            })
                        );
                    }
                }
            }
        }
    };
    
    // Helper patterns for type conversions
    
    // Spec types (for YAML deserialization)
    (@spec_type Vec<$crate::engine::NodeId>) => { Vec<String> };
    (@spec_type $crate::engine::NodeId) => { String };
    (@spec_type $t:ty) => { $t };
    
    // Python field types
    (@py_field_type Vec<$crate::engine::NodeId>) => { Vec<pyo3::PyObject> };
    (@py_field_type $crate::engine::NodeId) => { pyo3::PyObject };
    (@py_field_type $t:ty) => { $t };
    
    // Build field from spec (for Box<dyn Node>)
    (@build_field $expr:expr, Vec<$crate::engine::NodeId>) => {
        {
            let mut nodes = Vec::new();
            for id in $expr {
                nodes.push($crate::engine::build_node(&serde_yaml::Value::String(id))?);
            }
            nodes
        }
    };
    (@build_field $expr:expr, $crate::engine::NodeId) => {
        $crate::engine::build_node(&serde_yaml::Value::String($expr))?
    };
    (@build_field $expr:expr, $t:ty) => { $expr };
    
    // Extract field from arena node
    (@extract_arena_field $spec:expr, $field:expr, Vec<$crate::engine::NodeId>) => {
        match $spec.fields.get($field) {
            Some($crate::engine::FieldValue::Many(ids)) => Ok(ids.clone()),
            _ => Err(format!("Expected Vec<NodeId> for field {}", $field)),
        }
    };
    (@extract_arena_field $spec:expr, $field:expr, $crate::engine::NodeId) => {
        match $spec.fields.get($field) {
            Some($crate::engine::FieldValue::One(id)) => Ok(*id),
            _ => Err(format!("Expected NodeId for field {}", $field)),
        }
    };
    (@extract_arena_field $spec:expr, $field:expr, f64) => {
        match $spec.fields.get($field) {
            Some($crate::engine::FieldValue::Float(f)) => Ok(*f),
            _ => Err(format!("Expected f64 for field {}", $field)),
        }
    };
    (@extract_arena_field $spec:expr, $field:expr, String) => {
        match $spec.fields.get($field) {
            Some($crate::engine::FieldValue::Str(s)) => Ok(s.clone()),
            _ => Err(format!("Expected String for field {}", $field)),
        }
    };
    
    // Runtime extraction for arena evaluation
    (@extract_arena_field_runtime $spec:expr, $field:expr, Vec<$crate::engine::NodeId>) => {
        match $spec.fields.get($field) {
            Some($crate::engine::FieldValue::Many(ids)) => Ok(ids.clone()),
            _ => Err(format!("Expected Vec<NodeId> for field {}", $field)),
        }
    };
    (@extract_arena_field_runtime $spec:expr, $field:expr, $crate::engine::NodeId) => {
        match $spec.fields.get($field) {
            Some($crate::engine::FieldValue::One(id)) => Ok(*id),
            _ => Err(format!("Expected NodeId for field {}", $field)),
        }
    };
    (@extract_arena_field_runtime $spec:expr, $field:expr, f64) => {
        match $spec.fields.get($field) {
            Some($crate::engine::FieldValue::Float(f)) => Ok(*f),
            _ => Err(format!("Expected f64 for field {}", $field)),
        }
    };
    (@extract_arena_field_runtime $spec:expr, $field:expr, String) => {
        match $spec.fields.get($field) {
            Some($crate::engine::FieldValue::Str(s)) => Ok(s.clone()),
            _ => Err(format!("Expected String for field {}", $field)),
        }
    };
}

/// Trait for Python module initialization
pub trait PyModuleInit {
    fn add_to_module(m: &pyo3::types::PyModule) -> pyo3::PyResult<()>;
}

/// Global registry for arena node evaluation
use std::sync::RwLock;
use std::collections::HashMap;
use once_cell::sync::Lazy;

type ArenaEvalFn = Box<dyn Fn(&$crate::engine::ArenaNode, &[f64], &HashMap<String, f64>) -> f64 + Send + Sync>;

pub static ARENA_NODE_BUILDERS: Lazy<RwLock<HashMap<&'static str, ArenaEvalFn>>> = 
    Lazy::new(|| RwLock::new(HashMap::new()));

pub struct ArenaEngineRegistry;

/// Initialize all nodes
#[macro_export]
macro_rules! register_all_nodes {
    ($($node:ident),* $(,)?) => {
        pub fn register_all_arena_nodes() {
            $(
                $crate::ArenaEngineRegistry::[<register_ $node:snake>]();
            )*
        }
        
        pub fn add_all_to_python_module(m: &pyo3::types::PyModule) -> pyo3::PyResult<()> {
            $(
                <$node as $crate::define_node::PyModuleInit>::add_to_module(m)?;
            )*
            Ok(())
        }
    };
}