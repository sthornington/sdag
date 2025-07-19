#[macro_use]
extern crate inventory;

use pyo3::prelude::*;
use std::collections::HashMap;

// Re-export the py_node macro
use py_node_macro::py_node;

// Our simple macro system
#[macro_use]
mod simple_node_macro;
use simple_node_macro::{EvalNode, ArenaEval};

// Engine module with arena types
mod engine;
use engine::{ArenaGraph, NodeId};

// Re-export for macro use
pub use crate as crate;

// ===========================================================================
// DEFINE ALL NODES WITH DEAD SIMPLE MACRO
// ===========================================================================

// Input node
define_simple_node!(
    Input,
    tag = "input",
    fields = { name: String }
);

impl EvalNode for InputNode {
    fn eval(&self, _values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        *inputs.get(&self.name).unwrap_or(&0.0)
    }
}

// Constant node
define_simple_node!(
    Const,
    tag = "const", 
    fields = { value: f64 }
);

impl EvalNode for ConstNode {
    fn eval(&self, _values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        self.value
    }
}

// Add node
define_simple_node!(
    Add,
    tag = "add",
    fields = { children: Vec<NodeId> }
);

impl EvalNode for AddNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|&id| values[id]).sum()
    }
}

// Multiply node
define_simple_node!(
    Mul,
    tag = "mul",
    fields = { children: Vec<NodeId> }
);

impl EvalNode for MulNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|&id| values[id]).product()
    }
}

// Divide node
define_simple_node!(
    Div,
    tag = "div",
    fields = { left: NodeId, right: NodeId }
);

impl EvalNode for DivNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        let l = values[self.left];
        let r = values[self.right];
        if r == 0.0 { f64::NAN } else { l / r }
    }
}

// ===========================================================================
// THAT'S IT! The rest is just the Graph/Sampler API
// ===========================================================================

/// Python Graph builder
#[pyclass]
pub struct Graph {
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
    
    // The node creation methods are added by the macro!
    
    fn freeze(&self, py: Python, root: PyObject) -> PyResult<String> {
        // ... existing freeze implementation ...
        // [keeping the same as before]
        freeze_graph(self, py, root)
    }
}

/// Python Sampler
#[pyclass]
struct Sampler {
    graph: String,
    outputs: Vec<usize>,
    engine_name: String,
}

#[pymethods]
impl Sampler {
    #[new]
    #[pyo3(signature = (graph, outputs, engine_name = "lazy"))]
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
        
        // Build nodes using our auto-registered builders
        let mut nodes: Vec<Box<dyn ArenaEval>> = Vec::new();
        for arena_node in &arena.nodes {
            let node = simple_node_macro::build_arena_node(arena_node)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
            nodes.push(node);
        }
        
        // Run evaluation with trigger-based output
        let mut results = Vec::new();
        let mut prev_trigger: Option<f64> = None;
        
        for row in rows {
            let mut values = vec![0.0; arena.nodes.len()];
            
            // Evaluate all nodes
            for i in 0..arena.nodes.len() {
                values[i] = nodes[i].eval_arena(&values, &row);
            }
            
            // Check trigger
            let trigger_val = values[arena.root];
            if prev_trigger.map_or(true, |p| p != trigger_val) {
                let mut record = HashMap::new();
                record.insert("trigger".to_string(), trigger_val);
                
                for (i, &output_id) in self.outputs.iter().enumerate() {
                    record.insert(format!("output{}", i), values[output_id]);
                }
                
                results.push(record);
                prev_trigger = Some(trigger_val);
            }
        }
        
        Ok(results)
    }
}

/// Python module
#[pymodule]
fn sdag(_py: Python, m: &PyModule) -> PyResult<()> {
    // Register all nodes automatically!
    simple_node_macro::register_all_nodes(m)?;
    
    m.add_class::<Graph>()?;
    m.add_class::<Sampler>()?;
    Ok(())
}

// Helper function for freeze (same as before)
fn freeze_graph(graph: &Graph, py: Python, root: PyObject) -> PyResult<String> {
    use pyo3::types::{PyList, PySequence};
    use serde_yaml::{Mapping, Value};
    
    // ... [keeping the same freeze implementation as before] ...
    
    // Discover reachable nodes
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
                    if let Ok(list) = val.downcast::<PyList>() {
                        for item in list.iter() {
                            if item.hasattr("id")? {
                                stack.push(item.extract()?);
                            }
                        }
                    } else if let Ok(child) = val.extract::<PyObject>() {
                        if child.as_ref(py).hasattr("id")? {
                            stack.push(child);
                        }
                    }
                }
            }
        }
    }
    
    seen.reverse();
    
    // Build YAML
    let mut id2idx = HashMap::new();
    for (i, sid) in seen.iter().enumerate() {
        id2idx.insert(sid.clone(), i);
    }
    
    let mut nodes_seq = Vec::new();
    for sid in &seen {
        let obj = graph.registry.get(sid)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(format!("Unknown node '{}'", sid)))?;
        
        let mut mapping = Mapping::new();
        mapping.insert(Value::String("id".into()), serde_yaml::to_value(id2idx[sid]).unwrap());
        
        let tag: String = obj.as_ref(py).get_type().getattr("TYPE")?.extract()?;
        mapping.insert(Value::String("type".into()), Value::String(tag));
        
        let fields: Vec<String> = obj.as_ref(py).get_type().getattr("FIELDS")?.extract()?;
        for field in fields {
            let val = obj.as_ref(py).getattr(field.as_str())?;
            let entry = if let Ok(list) = val.downcast::<PyList>() {
                let mut idxs = Vec::new();
                for item in list.iter() {
                    let child: PyObject = item.extract()?;
                    let cid: String = child.as_ref(py).getattr("id")?.extract()?;
                    idxs.push(Value::Number(serde_yaml::Number::from(id2idx[&cid] as i64)));
                }
                Value::Sequence(idxs)
            } else if let Ok(child) = val.extract::<PyObject>() {
                if child.as_ref(py).hasattr("id")? {
                    let cid: String = child.as_ref(py).getattr("id")?.extract()?;
                    Value::Number(serde_yaml::Number::from(id2idx[&cid] as i64))
                } else if let Ok(s) = val.extract::<String>() {
                    Value::String(s)
                } else if let Ok(f) = val.extract::<f64>() {
                    serde_yaml::to_value(f).unwrap()
                } else {
                    continue;
                }
            } else {
                continue;
            };
            mapping.insert(Value::String(field), entry);
        }
        
        nodes_seq.push(Value::Mapping(mapping));
    }
    
    let mut top = Mapping::new();
    top.insert(Value::String("nodes".into()), Value::Sequence(nodes_seq));
    top.insert(Value::String("root".into()), Value::Number(serde_yaml::Number::from(*id2idx.get(&root_str).unwrap() as i64)));
    
    Ok(serde_yaml::to_string(&Value::Mapping(top))?.trim_end_matches('\n').to_string())
}