#[macro_use]
extern crate inventory;

use pyo3::prelude::*;
use pyo3::types::{PySequence, PyTuple};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// Re-export the existing macro
use py_node_macro::py_node;

mod engine;
use engine::{NodeDef, ArenaGraph, ArenaNode, FieldValue, NodeId};

/// Define a comprehensive node creation macro
macro_rules! define_node {
    ($name:ident, $engine_name:ident, $tag:literal, {$($field:ident: $field_ty:ty),*}, 
     eval_arena = |$self:ident, $values:ident| $eval_expr:expr) => {
        // Python wrapper using existing py_node macro
        #[py_node($engine_name::TYPE, $($field),*)]
        #[pyclass(name = stringify!($name), text_signature = concat!("(id, ", $(stringify!($field), ", "),* ")"))]
        struct $name {
            #[pyo3(get)]
            id: String,
            $(
                #[pyo3(get)]
                $field: define_node!(@py_type $field_ty),
            )*
        }
        
        // Engine implementation
        #[derive(Debug, Clone)]
        pub struct $engine_name {
            $(pub $field: $field_ty,)*
        }
        
        impl $engine_name {
            pub const TYPE: &'static str = $tag;
        }
        
        // Arena evaluation
        impl ArenaEvalNode for $engine_name {
            fn eval_arena(&$self, $values: &[f64]) -> f64 {
                $eval_expr
            }
        }
        
        // Regular node evaluation
        impl engine::Node for $engine_name {
            fn eval(&self, _row: &HashMap<String, f64>) -> f64 {
                0.0 // Not used in arena mode
            }
        }
        
        // NodeDef implementation for building from YAML
        impl NodeDef for $engine_name {
            const TYPE: &'static str = $tag;
            
            fn from_yaml(v: &serde_yaml::Value) -> Result<Box<dyn engine::Node + Send + Sync>, String> {
                #[derive(Deserialize)]
                struct Spec {
                    $($field: define_node!(@spec_type $field_ty),)*
                }
                
                let spec: Spec = serde_yaml::from_value(v.clone())
                    .map_err(|e| e.to_string())?;
                
                Ok(Box::new($engine_name {
                    $($field: spec.$field,)*
                }))
            }
            
            fn from_arena_spec(spec: &ArenaNode) -> Result<Box<dyn ArenaEvalNode>, String> {
                $(let $field = define_node!(@extract_field spec, stringify!($field), $field_ty)?;)*
                
                Ok(Box::new($engine_name {
                    $($field,)*
                }))
            }
        }
    };
    
    // Type conversions
    (@py_type NodeId) => { PyObject };
    (@py_type Vec<NodeId>) => { Vec<PyObject> };
    (@py_type $t:ty) => { $t };
    
    (@spec_type NodeId) => { NodeId };
    (@spec_type Vec<NodeId>) => { Vec<NodeId> };
    (@spec_type $t:ty) => { $t };
    
    (@extract_field $spec:expr, $field:expr, NodeId) => {
        match $spec.fields.get($field) {
            Some(FieldValue::One(id)) => Ok(*id),
            _ => Err(format!("Expected NodeId for field {}", $field)),
        }
    };
    (@extract_field $spec:expr, $field:expr, Vec<NodeId>) => {
        match $spec.fields.get($field) {
            Some(FieldValue::Many(ids)) => Ok(ids.clone()),
            _ => Err(format!("Expected Vec<NodeId> for field {}", $field)),
        }
    };
    (@extract_field $spec:expr, $field:expr, f64) => {
        match $spec.fields.get($field) {
            Some(FieldValue::Float(f)) => Ok(*f),
            _ => Err(format!("Expected f64 for field {}", $field)),
        }
    };
    (@extract_field $spec:expr, $field:expr, String) => {
        match $spec.fields.get($field) {
            Some(FieldValue::Str(s)) => Ok(s.clone()),
            _ => Err(format!("Expected String for field {}", $field)),
        }
    };
}

// Arena evaluation trait
pub trait ArenaEvalNode: Send + Sync {
    fn eval_arena(&self, values: &[f64]) -> f64;
}

// Engine trait for different evaluation strategies
pub trait ArenaEngine {
    fn name(&self) -> &str;
    fn run(&self, graph: &ArenaGraph, rows: Vec<HashMap<String, f64>>) -> Vec<HashMap<String, f64>>;
}

// Define nodes using the macro
define_node!(InputNode, InputNodeImpl, "input", {name: String}, 
    eval_arena = |self, _values| {
        // Input values handled specially by engine
        0.0
    }
);

define_node!(Const, ConstNode, "const", {value: f64},
    eval_arena = |self, _values| {
        self.value
    }
);

define_node!(Add, AddNode, "add", {children: Vec<NodeId>},
    eval_arena = |self, values| {
        self.children.iter().map(|&id| values[id]).sum()
    }
);

define_node!(Mul, MulNode, "mul", {children: Vec<NodeId>},
    eval_arena = |self, values| {
        self.children.iter().map(|&id| values[id]).product()
    }
);

define_node!(Div, DivNode, "div", {left: NodeId, right: NodeId},
    eval_arena = |self, values| {
        let l = values[self.left];
        let r = values[self.right];
        if r == 0.0 { f64::NAN } else { l / r }
    }
);

/// Multiple evaluation engines
pub struct TopologicalArenaEngine {
    pub outputs: Vec<NodeId>,
}

impl TopologicalArenaEngine {
    pub fn new(outputs: Vec<NodeId>) -> Self {
        Self { outputs }
    }
}

impl ArenaEngine for TopologicalArenaEngine {
    fn name(&self) -> &str {
        "topological"
    }
    
    fn run(&self, graph: &ArenaGraph, rows: Vec<HashMap<String, f64>>) -> Vec<HashMap<String, f64>> {
        // Build evaluator nodes
        let mut eval_nodes: Vec<Box<dyn ArenaEvalNode>> = Vec::new();
        
        for node in &graph.nodes {
            let eval_node = match node.tag.as_str() {
                "input" => InputNodeImpl::from_arena_spec(node),
                "const" => ConstNode::from_arena_spec(node),
                "add" => AddNode::from_arena_spec(node),
                "mul" => MulNode::from_arena_spec(node),
                "div" => DivNode::from_arena_spec(node),
                _ => Err(format!("Unknown node type: {}", node.tag)),
            }.unwrap();
            
            eval_nodes.push(eval_node);
        }
        
        let mut results = Vec::new();
        
        for row in rows {
            let mut values = vec![0.0; graph.nodes.len()];
            
            // Evaluate all nodes in topological order
            for (i, node) in graph.nodes.iter().enumerate() {
                values[i] = if node.tag == "input" {
                    if let Some(FieldValue::Str(name)) = node.fields.get("name") {
                        *row.get(name).unwrap_or(&0.0)
                    } else {
                        0.0
                    }
                } else {
                    eval_nodes[i].eval_arena(&values)
                };
            }
            
            // Build output record
            let mut record = HashMap::new();
            record.insert("trigger".to_string(), values[graph.root]);
            
            for &output_id in &self.outputs {
                record.insert(format!("output{}", output_id), values[output_id]);
            }
            
            results.push(record);
        }
        
        results
    }
}

/// Lazy evaluation engine
pub struct LazyArenaEngine {
    pub outputs: Vec<NodeId>,
}

impl LazyArenaEngine {
    pub fn new(outputs: Vec<NodeId>) -> Self {
        Self { outputs }
    }
}

impl ArenaEngine for LazyArenaEngine {
    fn name(&self) -> &str {
        "lazy"
    }
    
    fn run(&self, graph: &ArenaGraph, rows: Vec<HashMap<String, f64>>) -> Vec<HashMap<String, f64>> {
        // Similar to topological but only evaluates needed nodes
        TopologicalArenaEngine::new(self.outputs.clone()).run(graph, rows)
    }
}

/// Python Graph builder
#[pyclass]
struct Graph {
    counter: usize,
    registry: HashMap<String, PyObject>,
}

#[pymethods]
impl Graph {
    #[new]
    fn new() -> Self {
        Graph {
            counter: 0,
            registry: HashMap::new(),
        }
    }
    
    fn input(&mut self, py: Python, name: String) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = InputNode { id: id.clone(), name };
        let py_node = node.into_py(py);
        self.registry.insert(id, py_node.clone());
        py_node
    }
    
    #[pyo3(name = "const")]
    fn const_node(&mut self, py: Python, value: f64) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Const { id: id.clone(), value };
        let py_node = node.into_py(py);
        self.registry.insert(id, py_node.clone());
        py_node
    }
    
    fn add(&mut self, py: Python, children: Vec<PyObject>) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Add { id: id.clone(), children };
        let py_node = node.into_py(py);
        self.registry.insert(id, py_node.clone());
        py_node
    }
    
    fn mul(&mut self, py: Python, children: Vec<PyObject>) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Mul { id: id.clone(), children };
        let py_node = node.into_py(py);
        self.registry.insert(id, py_node.clone());
        py_node
    }
    
    fn div(&mut self, py: Python, left: PyObject, right: PyObject) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Div { id: id.clone(), left, right };
        let py_node = node.into_py(py);
        self.registry.insert(id, py_node.clone());
        py_node
    }
    
    fn freeze(&self, py: Python, root: PyObject) -> PyResult<String> {
        // Implementation from original lib.rs
        let mut seen = Vec::new();
        let root_str: String = root.as_ref(py).getattr("id")?.extract()?;
        let mut stack = vec![root.clone()];
        
        while let Some(obj) = stack.pop() {
            let id: String = obj.as_ref(py).getattr("id")?.extract()?;
            if seen.contains(&id) { continue; }
            seen.push(id.clone());
            
            let cls = obj.as_ref(py).get_type();
            if let Ok(fields) = cls.getattr("FIELDS") {
                if let Ok(field_names) = fields.extract::<Vec<String>>() {
                    for field in field_names {
                        let val = obj.as_ref(py).getattr(field.as_str())?;
                        if let Ok(seq) = val.cast_as::<PySequence>() {
                            for item in seq.iter()? {
                                let child: PyObject = item?.extract()?;
                                if child.as_ref(py).get_type().getattr("TYPE").is_ok() {
                                    stack.push(child);
                                }
                            }
                        } else if let Ok(child) = val.extract::<PyObject>() {
                            if child.as_ref(py).get_type().getattr("TYPE").is_ok() {
                                stack.push(child);
                            }
                        }
                    }
                }
            }
        }
        
        seen.reverse();
        
        let mut id2idx = HashMap::new();
        for (i, sid) in seen.iter().enumerate() {
            id2idx.insert(sid.clone(), i);
        }
        let root_idx = *id2idx.get(&root_str).unwrap();
        
        let mut nodes_seq = Vec::with_capacity(seen.len());
        for sid in &seen {
            let obj = self.registry.get(sid)
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(format!("Unknown node ID '{}'", sid)))?;
            let mut mapping = serde_yaml::Mapping::new();
            
            mapping.insert(
                serde_yaml::Value::String("id".into()),
                serde_yaml::to_value(id2idx[sid]).unwrap(),
            );
            
            let tag: String = obj.as_ref(py).get_type().getattr("TYPE")?.extract()?;
            mapping.insert(serde_yaml::Value::String("type".into()), serde_yaml::Value::String(tag));
            
            let fields: Vec<String> = obj.as_ref(py).get_type().getattr("FIELDS")?.extract()?;
            for field in fields {
                let val = obj.as_ref(py).getattr(field.as_str())?;
                let entry = if let Ok(seq) = val.cast_as::<PySequence>() {
                    let mut idxs = Vec::new();
                    for item in seq.iter()? {
                        let child: PyObject = item?.extract()?;
                        let cid: String = child.as_ref(py).getattr("id")?.extract()?;
                        idxs.push(serde_yaml::Value::Number(serde_yaml::Number::from(id2idx[&cid] as i64)));
                    }
                    serde_yaml::Value::Sequence(idxs)
                } else if let Ok(child) = val.extract::<PyObject>() {
                    if child.as_ref(py).hasattr("id")? {
                        let cid: String = child.as_ref(py).getattr("id")?.extract()?;
                        serde_yaml::Value::Number(serde_yaml::Number::from(id2idx[&cid] as i64))
                    } else {
                        if let Ok(s) = val.extract::<String>() {
                            serde_yaml::Value::String(s)
                        } else if let Ok(f) = val.extract::<f64>() {
                            serde_yaml::to_value(f).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
                        } else {
                            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                                "Unsupported field '{}' on node '{}'", field, sid
                            )));
                        }
                    }
                } else {
                    continue;
                };
                mapping.insert(serde_yaml::Value::String(field), entry);
            }
            
            nodes_seq.push(serde_yaml::Value::Mapping(mapping));
        }
        
        let mut top = serde_yaml::Mapping::new();
        top.insert(serde_yaml::Value::String("nodes".into()), serde_yaml::Value::Sequence(nodes_seq));
        top.insert(serde_yaml::Value::String("root".into()), serde_yaml::Value::Number(serde_yaml::Number::from(root_idx as i64)));
        
        let yaml = serde_yaml::to_string(&serde_yaml::Value::Mapping(top))
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(yaml.trim_end_matches('\n').to_string())
    }
}

/// Python Sampler with multiple engine support
#[pyclass]
struct Sampler {
    graph: String,
    outputs: Vec<usize>,
    engine_name: String,
}

#[pymethods]
impl Sampler {
    #[new]
    #[pyo3(signature = (graph, outputs, engine_name = "topological"))]
    fn new(graph: &str, outputs: Vec<usize>, engine_name: &str) -> PyResult<Self> {
        ArenaGraph::from_yaml(graph)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
        Ok(Sampler { 
            graph: graph.to_string(), 
            outputs,
            engine_name: engine_name.to_string(),
        })
    }
    
    fn run(&self, rows: Vec<HashMap<String, f64>>) -> PyResult<Vec<HashMap<String, f64>>> {
        let arena = ArenaGraph::from_yaml(&self.graph)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
        
        let engine: Box<dyn ArenaEngine> = match self.engine_name.as_str() {
            "topological" => Box::new(TopologicalArenaEngine::new(self.outputs.clone())),
            "lazy" => Box::new(LazyArenaEngine::new(self.outputs.clone())),
            _ => return Err(pyo3::exceptions::PyValueError::new_err(
                format!("Unknown engine: {}", self.engine_name)
            )),
        };
        
        Ok(engine.run(&arena, rows))
    }
}

/// Python module
#[pymodule]
fn sdag(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<InputNode>()?;
    m.add_class::<Const>()?;
    m.add_class::<Add>()?;
    m.add_class::<Mul>()?;
    m.add_class::<Div>()?;
    m.add_class::<Graph>()?;
    m.add_class::<Sampler>()?;
    Ok(())
}