/// Comprehensive macro for defining nodes with all boilerplate
#[macro_export]
macro_rules! define_dag_node {
    (
        $node_name:ident {
            type_tag: $tag:literal,
            fields: { $($field:ident : $field_ty:ty),* $(,)? },
            eval_row: |$self1:ident, $row:ident| $eval_row:expr,
            eval_arena: |$self2:ident, $values:ident, $inputs:ident| $eval_arena:expr $(,)?
        }
    ) => {
        // Use paste for identifier manipulation
        paste::paste! {
            // Core node struct
            #[derive(Debug, Clone)]
            pub struct $node_name {
                $(pub $field: define_dag_node!(@convert_field_type $field_ty),)*
            }
            
            impl $node_name {
                pub const TYPE: &'static str = $tag;
            }
            
            // Implement evaluation trait
            impl $crate::engine_traits::EvalNode for $node_name {
                fn eval_row(&self, $row: &std::collections::HashMap<String, f64>) -> f64 {
                    let $self1 = self;
                    $eval_row
                }
                
                fn eval_arena(&self, $values: &[f64], $inputs: &std::collections::HashMap<String, f64>) -> f64 {
                    let $self2 = self;
                    $eval_arena
                }
            }
            
            // Node builder
            pub struct [<$node_name Builder>];
            
            impl $crate::engine_traits::NodeBuilder for [<$node_name Builder>] {
                fn build(&self, node: &$crate::arena::ArenaNode) -> Result<Box<dyn $crate::engine_traits::EvalNode>, String> {
                    use serde::Deserialize;
                    
                    #[derive(Deserialize)]
                    struct Fields {
                        $($field: define_dag_node!(@serde_field_type $field_ty),)*
                    }
                    
                    let fields: Fields = serde_yaml::from_value(node.data.clone())
                        .map_err(|e| format!("Failed to parse {} fields: {}", $tag, e))?;
                    
                    Ok(Box::new($node_name {
                        $($field: define_dag_node!(@convert_from_serde fields.$field, $field_ty),)*
                    }))
                }
            }
            
            // Python wrapper class
            #[pyo3::pyclass(name = stringify!($node_name))]
            #[derive(Clone, Debug)]
            pub struct [<$node_name Py>] {
                #[pyo3(get)]
                pub id: String,
                $(
                    #[pyo3(get)]
                    pub $field: define_dag_node!(@py_field_type $field_ty),
                )*
            }
            
            #[pyo3::pymethods]
            impl [<$node_name Py>] {
                #[classattr]
                const TYPE: &'static str = $tag;
                
                #[classattr]
                const FIELDS: &'static [&'static str] = &[$(stringify!($field)),*];
                
                #[new]
                #[pyo3(signature = (id, $($field),*))]
                pub fn new(id: String, $($field: define_dag_node!(@py_field_type $field_ty)),*) -> Self {
                    Self { id, $($field),* }
                }
            }
            
            // Registration function
            pub fn [<register_ $node_name:snake>](registry: &mut $crate::engine_traits::NodeRegistry) {
                registry.register($tag, Box::new([<$node_name Builder>]));
            }
            
            // Python module registration
            pub fn [<register_ $node_name:snake _py>](m: &pyo3::types::PyModule) -> pyo3::PyResult<()> {
                m.add_class::<[<$node_name Py>]>()?;
                Ok(())
            }
        }
    };
    
    // Field type conversions
    (@convert_field_type $crate::arena::NodeId) => { $crate::arena::NodeId };
    (@convert_field_type Vec<$crate::arena::NodeId>) => { Vec<$crate::arena::NodeId> };
    (@convert_field_type $t:ty) => { $t };
    
    (@serde_field_type $crate::arena::NodeId) => { usize };
    (@serde_field_type Vec<$crate::arena::NodeId>) => { Vec<usize> };
    (@serde_field_type $t:ty) => { $t };
    
    (@py_field_type $crate::arena::NodeId) => { pyo3::PyObject };
    (@py_field_type Vec<$crate::arena::NodeId>) => { Vec<pyo3::PyObject> };
    (@py_field_type $t:ty) => { $t };
    
    (@convert_from_serde $expr:expr, $crate::arena::NodeId) => { $expr };
    (@convert_from_serde $expr:expr, Vec<$crate::arena::NodeId>) => { $expr };
    (@convert_from_serde $expr:expr, $t:ty) => { $expr };
}