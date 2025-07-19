use pyo3::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ===========================================================================
// TYPES
// ===========================================================================

pub type NodeId = usize;

// Single node enum - all nodes in one place
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Node {
    Input { name: String },
    Const { value: f64 },
    Add { children: Vec<NodeId> },
    Mul { children: Vec<NodeId> },
    Div { left: NodeId, right: NodeId },
}

// The graph structure
#[derive(Serialize, Deserialize)]
pub struct GraphData {
    nodes: Vec<Node>,
    root: NodeId,
}

// ===========================================================================
// PYTHON INTERFACE - Using a single generic node class
// ===========================================================================

#[pyclass]
#[derive(Clone)]
pub struct PyNode {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub node_type: String,
    pub data: Py<PyAny>,  // Stores the actual data as a Python dict
}

#[pyclass]
pub struct Graph {
    next_id: NodeId,
    registry: HashMap<String, PyNode>,
}

#[pymethods]
impl Graph {
    #[new]
    fn new() -> Self {
        Self {
            next_id: 0,
            registry: HashMap::new(),
        }
    }
    
    fn input(&mut self, py: Python, name: String) -> PyNode {
        self.create_node(py, "input", [("name", name.to_object(py))].into_py_dict(py))
    }
    
    #[pyo3(name = "const")]
    fn const_(&mut self, py: Python, value: f64) -> PyNode {
        self.create_node(py, "const", [("value", value.to_object(py))].into_py_dict(py))
    }
    
    fn add(&mut self, py: Python, children: Vec<PyObject>) -> PyNode {
        self.create_node(py, "add", [("children", children.to_object(py))].into_py_dict(py))
    }
    
    fn mul(&mut self, py: Python, children: Vec<PyObject>) -> PyNode {
        self.create_node(py, "mul", [("children", children.to_object(py))].into_py_dict(py))
    }
    
    fn div(&mut self, py: Python, left: PyObject, right: PyObject) -> PyNode {
        let data = [("left", left), ("right", right)].into_py_dict(py);
        self.create_node(py, "div", data)
    }
    
    fn create_node(&mut self, py: Python, node_type: &str, data: &PyDict) -> PyNode {
        let id = format!("n{}", self.next_id);
        self.next_id += 1;
        
        let node = PyNode {
            id: id.clone(),
            node_type: node_type.to_string(),
            data: data.into(),
        };
        
        self.registry.insert(id, node.clone());
        node
    }
    
    fn freeze(&self, py: Python, root: PyNode) -> PyResult<String> {
        // Collect all nodes via traversal
        let mut seen = Vec::new();
        let mut stack = vec![root];
        
        while let Some(node) = stack.pop() {
            if seen.iter().any(|n: &PyNode| n.id == node.id) {
                continue;
            }
            
            // Add children to stack based on node type
            let data: &PyDict = node.data.as_ref(py).downcast()?;
            match node.node_type.as_str() {
                "add" | "mul" => {
                    if let Ok(children) = data.get_item("children") {
                        let children: Vec<PyNode> = children.extract()?;
                        stack.extend(children);
                    }
                }
                "div" => {
                    if let Ok(left) = data.get_item("left") {
                        stack.push(left.extract()?);
                    }
                    if let Ok(right) = data.get_item("right") {
                        stack.push(right.extract()?);
                    }
                }
                _ => {}
            }
            
            seen.push(node);
        }
        
        // Build serialized graph
        seen.reverse();
        let mut id_map: HashMap<String, NodeId> = HashMap::new();
        let mut nodes = Vec::new();
        let root_idx = seen.iter().position(|n| n.id == root.id).unwrap();
        
        for (idx, py_node) in seen.iter().enumerate() {
            id_map.insert(py_node.id.clone(), idx);
            
            let data: &PyDict = py_node.data.as_ref(py).downcast()?;
            let node = match py_node.node_type.as_str() {
                "input" => {
                    let name: String = data.get_item("name").unwrap().extract()?;
                    Node::Input { name }
                }
                "const" => {
                    let value: f64 = data.get_item("value").unwrap().extract()?;
                    Node::Const { value }
                }
                "add" => {
                    let children: Vec<PyNode> = data.get_item("children").unwrap().extract()?;
                    let children = children.iter()
                        .map(|c| id_map[&c.id])
                        .collect();
                    Node::Add { children }
                }
                "mul" => {
                    let children: Vec<PyNode> = data.get_item("children").unwrap().extract()?;
                    let children = children.iter()
                        .map(|c| id_map[&c.id])
                        .collect();
                    Node::Mul { children }
                }
                "div" => {
                    let left: PyNode = data.get_item("left").unwrap().extract()?;
                    let right: PyNode = data.get_item("right").unwrap().extract()?;
                    Node::Div {
                        left: id_map[&left.id],
                        right: id_map[&right.id],
                    }
                }
                _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    format!("Unknown node type: {}", py_node.node_type)
                )),
            };
            
            nodes.push(node);
        }
        
        let graph = GraphData { nodes, root: root_idx };
        serde_yaml::to_string(&graph)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
}

// ===========================================================================
// EVALUATION
// ===========================================================================

#[pyclass]
pub struct Sampler {
    nodes: Vec<Node>,
    root: NodeId,
    outputs: Vec<NodeId>,
}

#[pymethods]
impl Sampler {
    #[new]
    fn new(yaml: &str, outputs: Vec<NodeId>, _engine: Option<&str>) -> PyResult<Self> {
        let graph: GraphData = serde_yaml::from_str(yaml)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        Ok(Self {
            nodes: graph.nodes,
            root: graph.root,
            outputs,
        })
    }
    
    fn run(&self, rows: Vec<HashMap<String, f64>>) -> PyResult<Vec<HashMap<String, f64>>> {
        let mut results = Vec::new();
        let mut prev_trigger: Option<f64> = None;
        
        for inputs in rows {
            // Simple sweep evaluation
            let mut values = vec![0.0; self.nodes.len()];
            
            for (i, node) in self.nodes.iter().enumerate() {
                values[i] = match node {
                    Node::Input { name } => *inputs.get(name).unwrap_or(&0.0),
                    Node::Const { value } => *value,
                    Node::Add { children } => children.iter().map(|&id| values[id]).sum(),
                    Node::Mul { children } => children.iter().map(|&id| values[id]).product(),
                    Node::Div { left, right } => {
                        let l = values[*left];
                        let r = values[*right];
                        if r == 0.0 { f64::NAN } else { l / r }
                    }
                };
            }
            
            // Trigger-based output
            let trigger = values[self.root];
            if prev_trigger.map_or(true, |p| p != trigger) {
                let mut record = HashMap::new();
                record.insert("trigger".to_string(), trigger);
                
                for (i, &output_id) in self.outputs.iter().enumerate() {
                    record.insert(format!("output{}", i), values[output_id]);
                }
                
                results.push(record);
                prev_trigger = Some(trigger);
            }
        }
        
        Ok(results)
    }
}

// ===========================================================================
// MODULE
// ===========================================================================

#[pymodule]
fn sdag(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Graph>()?;
    m.add_class::<Sampler>()?;
    m.add_class::<PyNode>()?;
    Ok(())
}

// ===========================================================================
// ADDING A NEW NODE TYPE
// ===========================================================================
// To add a new node type:
// 1. Add variant to Node enum
// 2. Add case to evaluation match in Sampler::run
// 3. Add method to Graph to create it
// 4. Add case to Graph::freeze to serialize it
// That's it!