/// Macro for generating complete node implementations with all boilerplate
#[macro_export]
macro_rules! define_node {
    (
        name: $name:ident,
        type_tag: $tag:literal,
        fields: {
            $($field:ident: $field_ty:ty),* $(,)?
        },
        eval: |$self:ident, $values:ident: &[$val_ty:ty]| $eval:block
    ) => {
        paste::paste! {
            // Engine-side node implementation
            #[derive(Debug, Clone)]
            pub struct [<$name Node>] {
                $(pub $field: $field_ty,)*
            }
            
            impl [<$name Node>] {
                pub const TYPE: &'static str = $tag;
            }
            
            // Arena-based evaluation trait
            impl $crate::engine::ArenaEvalNode for [<$name Node>] {
                fn eval(&$self, $values: &[$val_ty]) -> f64 $eval
            }
            
            // YAML spec structure for deserialization
            #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            pub struct [<$name Spec>] {
                pub id: $crate::engine::NodeId,
                $(pub $field: define_node!(@spec_type $field_ty),)*
            }
            
            // Node definition trait for building from YAML
            impl $crate::engine::NodeDef for [<$name Spec>] {
                const TYPE: &'static str = $tag;
                
                fn from_yaml(v: &serde_yaml::Value) -> Result<Box<dyn $crate::engine::Node + Send + Sync>, String> {
                    let spec: Self = serde_yaml::from_value(v.clone())
                        .map_err(|e| e.to_string())?;
                    
                    // Build node from spec
                    $(let $field = define_node!(@build_field spec.$field, $field_ty);)*
                    
                    let node = [<$name Node>] { $($field),* };
                    Ok(Box::new(node))
                }
                
                fn from_arena_spec(spec: &$crate::engine::ArenaNode) -> Result<Box<dyn $crate::engine::ArenaEvalNode>, String> {
                    $(let $field = define_node!(@extract_field spec, stringify!($field), $field_ty)?;)*
                    
                    let node = [<$name Node>] { $($field),* };
                    Ok(Box::new(node))
                }
            }
            
            // Register with inventory
            inventory::submit! {
                $crate::engine::NodeBuilder { 
                    tag: $tag, 
                    build: [<$name Spec>]::from_yaml,
                    build_arena: [<$name Spec>]::from_arena_spec,
                }
            }
            
            // Python wrapper class
            #[pyclass(name = $name)]
            #[derive(Clone, Debug)]
            pub struct [<$name Py>] {
                #[pyo3(get)]
                pub id: String,
                $(
                    #[pyo3(get)]
                    pub $field: define_node!(@py_type $field_ty),
                )*
            }
            
            #[pymethods]
            impl [<$name Py>] {
                #[classattr]
                pub const TYPE: &'static str = $tag;
                
                #[classattr]
                pub const FIELDS: &'static [&'static str] = &[$(stringify!($field)),*];
                
                #[new]
                #[pyo3(signature = (id, $($field),*))]
                pub fn new(id: String, $($field: define_node!(@py_type $field_ty)),*) -> Self {
                    Self { id, $($field),* }
                }
            }
            
            // Add to module registration  
            impl $crate::ModuleInit {
                paste::paste! {
                    pub fn [<add_ $name:snake>](m: &pyo3::types::PyModule) -> pyo3::PyResult<()> {
                        m.add_class::<[<$name Py>]>()?;
                        Ok(())
                    }
                }
            }
        }
    };
    
    // Helper patterns for type conversions
    (@spec_type NodeId) => { $crate::engine::NodeId };
    (@spec_type Vec<NodeId>) => { Vec<$crate::engine::NodeId> };
    (@spec_type $t:ty) => { $t };
    
    (@py_type NodeId) => { PyObject };
    (@py_type Vec<NodeId>) => { Vec<PyObject> };
    (@py_type $t:ty) => { $t };
    
    (@build_field $expr:expr, NodeId) => {
        $crate::engine::build_node(&serde_yaml::Value::String($expr.clone()))?
    };
    (@build_field $expr:expr, Vec<NodeId>) => {
        {
            let mut out = Vec::new();
            for id in $expr.clone() {
                out.push($crate::engine::build_node(&serde_yaml::Value::String(id))?);
            }
            out
        }
    };
    (@build_field $expr:expr, $t:ty) => { $expr.clone() };
    
    (@extract_field $spec:expr, $field:expr, NodeId) => {
        match $spec.fields.get($field) {
            Some($crate::engine::FieldValue::One(id)) => Ok(*id),
            _ => Err(format!("Expected NodeId for field {}", $field))
        }
    };
    (@extract_field $spec:expr, $field:expr, Vec<NodeId>) => {
        match $spec.fields.get($field) {
            Some($crate::engine::FieldValue::Many(ids)) => Ok(ids.clone()),
            _ => Err(format!("Expected Vec<NodeId> for field {}", $field))
        }
    };
    (@extract_field $spec:expr, $field:expr, f64) => {
        match $spec.fields.get($field) {
            Some($crate::engine::FieldValue::Float(f)) => Ok(*f),
            _ => Err(format!("Expected f64 for field {}", $field))
        }
    };
    (@extract_field $spec:expr, $field:expr, String) => {
        match $spec.fields.get($field) {
            Some($crate::engine::FieldValue::Str(s)) => Ok(s.clone()),
            _ => Err(format!("Expected String for field {}", $field))
        }
    };
}