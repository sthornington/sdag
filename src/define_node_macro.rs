/// Comprehensive macro for defining nodes with all boilerplate
#[macro_export]
macro_rules! define_node {
    (
        $node_name:ident,
        type_tag = $tag:literal,
        fields = { $($field:ident : $field_ty:ty),* $(,)? },
        eval_arena = |$self:ident, $values:ident| $eval:expr
    ) => {
        // Import needed for macro
        use $crate::engine::{NodeId, ArenaEvalNode, NodeDef, Node, ArenaNode, FieldValue};
        use pyo3::prelude::*;
        
        // Engine node implementation
        #[derive(Debug, Clone)]
        pub struct $node_name {
            $(pub $field: $field_ty,)*
        }
        
        impl $node_name {
            pub const TYPE: &'static str = $tag;
        }
        
        // Regular Node trait for backward compatibility
        impl Node for $node_name {
            fn eval(&self, row: &std::collections::HashMap<String, f64>) -> f64 {
                // Default implementation - override if needed
                0.0
            }
        }
        
        // Arena evaluation
        impl ArenaEvalNode for $node_name {
            fn eval(&$self, $values: &[f64]) -> f64 {
                $eval
            }
        }
        
        // Helper to convert types for Python
        define_node!(@impl_python_wrapper $node_name, $tag, { $($field : $field_ty),* });
    };
    
    // Python wrapper implementation
    (@impl_python_wrapper $node_name:ident, $tag:literal, { $($field:ident : $field_ty:ty),* }) => {
        paste::paste! {
            #[pyclass(name = stringify!($node_name))]
            #[derive(Clone, Debug)]
            pub struct [<$node_name Py>] {
                #[pyo3(get)]
                pub id: String,
                $(
                    #[pyo3(get)]
                    pub $field: define_node!(@py_field_type $field_ty),
                )*
            }
            
            #[pymethods]
            impl [<$node_name Py>] {
                #[classattr]
                pub const TYPE: &'static str = $tag;
                
                #[classattr]
                pub const FIELDS: &'static [&'static str] = &[$(stringify!($field)),*];
                
                #[new]
                #[pyo3(signature = (id, $($field),*))]
                pub fn new(id: String, $($field: define_node!(@py_field_type $field_ty)),*) -> Self {
                    Self { id, $($field),* }
                }
            }
        }
    };
    
    // Type conversions for Python
    (@py_field_type NodeId) => { PyObject };
    (@py_field_type Vec<NodeId>) => { Vec<PyObject> };
    (@py_field_type f64) => { f64 };
    (@py_field_type String) => { String };
    (@py_field_type $t:ty) => { $t };
}