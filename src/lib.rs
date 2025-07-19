extern crate inventory;

use pyo3::prelude::*;
use std::collections::HashMap;

// Re-export the py_node macro
pub use py_node_macro::py_node;

// Our simple macro system
#[macro_use]
mod simple_node_macro;
use simple_node_macro::{EvalNode, ArenaEval};

// Engine module with arena types
mod engine;
use engine::ArenaGraph;

// Re-export for macro use
pub use engine::NodeId;


// ===========================================================================
// MANUAL NODE DEFINITIONS - A simple approach
// ===========================================================================

// Input node
#[derive(Debug, Clone)]
pub struct InputNode {
    pub name: String,
}

impl EvalNode for InputNode {
    fn eval(&self, _values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        *inputs.get(&self.name).unwrap_or(&0.0)
    }
}

impl ArenaEval for InputNode {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.eval(values, inputs)
    }
}

// Constant node
#[derive(Debug, Clone)]
pub struct ConstNode {
    pub value: f64,
}

impl EvalNode for ConstNode {
    fn eval(&self, _values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        self.value
    }
}

impl ArenaEval for ConstNode {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.eval(values, inputs)
    }
}

// Add node
#[derive(Debug, Clone)]
pub struct AddNode {
    pub children: Vec<NodeId>,
}

impl EvalNode for AddNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|&id| values[id]).sum()
    }
}

impl ArenaEval for AddNode {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.eval(values, inputs)
    }
}

// Multiply node
#[derive(Debug, Clone)]
pub struct MulNode {
    pub children: Vec<NodeId>,
}

impl EvalNode for MulNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|&id| values[id]).product()
    }
}

impl ArenaEval for MulNode {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.eval(values, inputs)
    }
}

// Divide node
#[derive(Debug, Clone)]
pub struct DivNode {
    pub left: NodeId,
    pub right: NodeId,
}

impl EvalNode for DivNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        let l = values[self.left];
        let r = values[self.right];
        if r == 0.0 { f64::NAN } else { l / r }
    }
}

impl ArenaEval for DivNode {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.eval(values, inputs)
    }
}

// ===========================================================================
// PYTHON BINDINGS
// ===========================================================================

// Python wrapper classes
#[pyclass]
pub struct Input {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub name: String,
}

#[pyclass(name = "Const")]
pub struct Const {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub value: f64,
}

#[pyclass]
pub struct Add {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub children: Vec<PyObject>,
}

#[pyclass]
pub struct Mul {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub children: Vec<PyObject>,
}

#[pyclass]
pub struct Div {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub left: PyObject,
    #[pyo3(get)]
    pub right: PyObject,
}

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
    
    fn input(&mut self, py: Python, name: String) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Input { id: id.clone(), name };
        let py_node = node.into_py(py);
        self.registry.insert(id, py_node.clone());
        py_node
    }
    
    fn const_(&mut self, py: Python, value: f64) -> PyObject {
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
        
        // Build nodes manually based on tag
        let mut nodes: Vec<Box<dyn ArenaEval>> = Vec::new();
        for arena_node in &arena.nodes {
            let node: Box<dyn ArenaEval> = match arena_node.tag.as_str() {
                "input" => {
                    let name = match arena_node.fields.get("name") {
                        Some(engine::FieldValue::Str(s)) => s.clone(),
                        _ => return Err(pyo3::exceptions::PyValueError::new_err("input node missing name")),
                    };
                    Box::new(InputNode { name })
                },
                "const" => {
                    let value = match arena_node.fields.get("value") {
                        Some(engine::FieldValue::Float(f)) => *f,
                        _ => return Err(pyo3::exceptions::PyValueError::new_err("const node missing value")),
                    };
                    Box::new(ConstNode { value })
                },
                "add" => {
                    let children = match arena_node.fields.get("children") {
                        Some(engine::FieldValue::Many(ids)) => ids.clone(),
                        _ => return Err(pyo3::exceptions::PyValueError::new_err("add node missing children")),
                    };
                    Box::new(AddNode { children })
                },
                "mul" => {
                    let children = match arena_node.fields.get("children") {
                        Some(engine::FieldValue::Many(ids)) => ids.clone(),
                        _ => return Err(pyo3::exceptions::PyValueError::new_err("mul node missing children")),
                    };
                    Box::new(MulNode { children })
                },
                "div" => {
                    let left = match arena_node.fields.get("left") {
                        Some(engine::FieldValue::One(id)) => *id,
                        _ => return Err(pyo3::exceptions::PyValueError::new_err("div node missing left")),
                    };
                    let right = match arena_node.fields.get("right") {
                        Some(engine::FieldValue::One(id)) => *id,
                        _ => return Err(pyo3::exceptions::PyValueError::new_err("div node missing right")),
                    };
                    Box::new(DivNode { left, right })
                },
                _ => return Err(pyo3::exceptions::PyValueError::new_err(format!("Unknown node type: {}", arena_node.tag))),
            };
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
    m.add_class::<Input>()?;
    m.add_class::<Const>()?;
    m.add_class::<Add>()?;
    m.add_class::<Mul>()?;
    m.add_class::<Div>()?;
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
        
        let node_type = get_node_type(py, &obj)?;
        match node_type.as_str() {
            "Add" | "Mul" => {
                let children: Vec<PyObject> = obj.as_ref(py).getattr("children")?.extract()?;
                for child in children {
                    stack.push(child);
                }
            },
            "Div" => {
                let left: PyObject = obj.as_ref(py).getattr("left")?.extract()?;
                let right: PyObject = obj.as_ref(py).getattr("right")?.extract()?;
                stack.push(left);
                stack.push(right);
            },
            _ => {},
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
        
        // Add fields based on node type
        match tag {
            "input" => {
                let name: String = obj.as_ref(py).getattr("name")?.extract()?;
                mapping.insert(Value::String("name".into()), Value::String(name));
            },
            "const" => {
                let value: f64 = obj.as_ref(py).getattr("value")?.extract()?;
                mapping.insert(Value::String("value".into()), serde_yaml::to_value(value).unwrap());
            },
            "add" | "mul" => {
                let children: Vec<PyObject> = obj.as_ref(py).getattr("children")?.extract()?;
                let mut idxs = Vec::new();
                for child in children {
                    let cid: String = child.as_ref(py).getattr("id")?.extract()?;
                    idxs.push(Value::Number(serde_yaml::Number::from(id2idx[&cid] as i64)));
                }
                mapping.insert(Value::String("children".into()), Value::Sequence(idxs));
            },
            "div" => {
                let left: PyObject = obj.as_ref(py).getattr("left")?.extract()?;
                let right: PyObject = obj.as_ref(py).getattr("right")?.extract()?;
                let lid: String = left.as_ref(py).getattr("id")?.extract()?;
                let rid: String = right.as_ref(py).getattr("id")?.extract()?;
                mapping.insert(Value::String("left".into()), Value::Number(serde_yaml::Number::from(id2idx[&lid] as i64)));
                mapping.insert(Value::String("right".into()), Value::Number(serde_yaml::Number::from(id2idx[&rid] as i64)));
            },
            _ => {},
        }
        
        nodes_seq.push(Value::Mapping(mapping));
    }
    
    let mut top = Mapping::new();
    top.insert(Value::String("nodes".into()), Value::Sequence(nodes_seq));
    top.insert(Value::String("root".into()), Value::Number(serde_yaml::Number::from(*id2idx.get(&root_str).unwrap() as i64)));
    
    serde_yaml::to_string(&Value::Mapping(top))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
        .map(|s| s.trim_end_matches('\n').to_string())
}