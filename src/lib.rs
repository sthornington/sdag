extern crate inventory;

use pyo3::prelude::*;
use std::collections::HashMap;

// Our simple macro system for traits
#[macro_use]
mod simple_node_macro;
use simple_node_macro::{EvalNode, ArenaEval};

// Engine module with arena types
mod engine;
use engine::ArenaGraph;

// Re-export for macro use
pub use engine::NodeId;

// Include the generated node definitions
mod generated_nodes;
use generated_nodes::{build_arena_node, register_nodes, freeze_node_fields};

// ===========================================================================
// MAIN STRUCTURES
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
    
    fn freeze(&self, py: Python, root: PyObject) -> PyResult<String> {
        freeze_graph(self, py, root)
    }
}

// The Graph node creation methods are auto-generated and appended to this impl block by build.rs

// The Graph methods are auto-generated in the included file

/// Python Sampler
#[pyclass]
struct Sampler {
    graph: String,
    outputs: Vec<usize>,
}

#[pymethods]
impl Sampler {
    #[new]
    #[pyo3(signature = (graph, outputs, _engine_name = "lazy"))]
    fn new(graph: &str, outputs: Vec<usize>, _engine_name: &str) -> PyResult<Self> {
        ArenaGraph::from_yaml(graph)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
        Ok(Sampler { 
            graph: graph.to_string(), 
            outputs,
        })
    }
    
    fn run(&self, rows: Vec<HashMap<String, f64>>) -> PyResult<Vec<HashMap<String, f64>>> {
        let arena = ArenaGraph::from_yaml(&self.graph)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
        
        // Build nodes using auto-generated builder
        let mut nodes: Vec<Box<dyn ArenaEval>> = Vec::new();
        for arena_node in &arena.nodes {
            let node = build_arena_node(arena_node)
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
    // Register all nodes using auto-generated function
    register_nodes(m)?;
    
    m.add_class::<Graph>()?;
    m.add_class::<Sampler>()?;
    Ok(())
}

// Helper function for freeze
fn freeze_graph(graph: &Graph, py: Python, root: PyObject) -> PyResult<String> {
    use serde_yaml::{Mapping, Value};
    
    // Helper to get node type
    fn get_node_type(py: Python, obj: &PyObject) -> PyResult<String> {
        let cls_name = obj.as_ref(py).get_type().name()?;
        Ok(cls_name.to_string())
    }
    
    // Discover reachable nodes
    let mut seen = Vec::new();
    let root_str: String = root.as_ref(py).getattr("id")?.extract()?;
    let mut stack = vec![root.clone()];
    
    while let Some(obj) = stack.pop() {
        let id: String = obj.as_ref(py).getattr("id")?.extract()?;
        if seen.contains(&id) { continue; }
        seen.push(id.clone());
        
        // Check if node has children
        if let Ok(children) = obj.as_ref(py).getattr("children") {
            if let Ok(children_vec) = children.extract::<Vec<PyObject>>() {
                for child in children_vec {
                    stack.push(child);
                }
            }
        }
        
        // Check for left/right (binary nodes)
        if let Ok(left) = obj.as_ref(py).getattr("left") {
            if let Ok(left_obj) = left.extract::<PyObject>() {
                stack.push(left_obj);
            }
        }
        if let Ok(right) = obj.as_ref(py).getattr("right") {
            if let Ok(right_obj) = right.extract::<PyObject>() {
                stack.push(right_obj);
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
        
        // Get node type (class name) and convert to tag
        let node_type = get_node_type(py, obj)?;
        let tag = match node_type.as_str() {
            "Input" => "input",
            "Const" => "const",
            "Add" => "add",
            "Mul" => "mul",
            "Div" => "div",
            _ => return Err(pyo3::exceptions::PyValueError::new_err(format!("Unknown node type: {}", node_type))),
        };
        mapping.insert(Value::String("type".into()), Value::String(tag.to_string()));
        
        // Extract fields using auto-generated helper
        freeze_node_fields(py, obj, tag, &mut mapping, &id2idx)?;
        
        nodes_seq.push(Value::Mapping(mapping));
    }
    
    let mut top = Mapping::new();
    top.insert(Value::String("nodes".into()), Value::Sequence(nodes_seq));
    top.insert(Value::String("root".into()), Value::Number(serde_yaml::Number::from(*id2idx.get(&root_str).unwrap() as i64)));
    
    serde_yaml::to_string(&Value::Mapping(top))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
        .map(|s| s.trim_end_matches('\n').to_string())
}