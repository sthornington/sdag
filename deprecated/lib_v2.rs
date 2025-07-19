use pyo3::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// Re-export for the derive macro
pub use sdag_derive::SdagNode;

// ===========================================================================
// CORE TYPES
// ===========================================================================

pub type NodeId = usize;

// The graph that builds and owns nodes
#[pyclass]
#[derive(Default)]
pub struct Graph {
    #[pyo3(get)]
    pub nodes: Vec<PyObject>,  // Python objects for node access
    arena: Vec<Box<dyn Node>>, // Actual computation nodes
}

// The serializable graph format
#[derive(Serialize, Deserialize)]
pub struct SerializedGraph {
    pub nodes: Vec<SerializedNode>,
    pub root: NodeId,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SerializedNode {
    #[serde(rename = "input")]
    Input { name: String },
    
    #[serde(rename = "const")]
    Const { value: f64 },
    
    #[serde(rename = "add")]
    Add { children: Vec<NodeId> },
    
    #[serde(rename = "mul")]
    Mul { children: Vec<NodeId> },
    
    #[serde(rename = "div")]
    Div { left: NodeId, right: NodeId },
}

// ===========================================================================
// NODE TRAIT
// ===========================================================================

pub trait Node: Send + Sync {
    fn eval(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64;
    fn as_serialized(&self) -> SerializedNode;
}

// ===========================================================================
// NODE IMPLEMENTATIONS
// ===========================================================================

#[derive(Clone, SdagNode)]
#[sdag(pyclass = "Input")]
pub struct InputNode {
    pub name: String,
}

impl Node for InputNode {
    fn eval(&self, _values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        *inputs.get(&self.name).unwrap_or(&0.0)
    }
    
    fn as_serialized(&self) -> SerializedNode {
        SerializedNode::Input { name: self.name.clone() }
    }
}

#[derive(Clone, SdagNode)]
#[sdag(pyclass = "Const")]
pub struct ConstNode {
    pub value: f64,
}

impl Node for ConstNode {
    fn eval(&self, _values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        self.value
    }
    
    fn as_serialized(&self) -> SerializedNode {
        SerializedNode::Const { value: self.value }
    }
}

#[derive(Clone, SdagNode)]
#[sdag(pyclass = "Add")]
pub struct AddNode {
    pub children: Vec<NodeId>,
}

impl Node for AddNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|&id| values[id]).sum()
    }
    
    fn as_serialized(&self) -> SerializedNode {
        SerializedNode::Add { children: self.children.clone() }
    }
}

#[derive(Clone, SdagNode)]
#[sdag(pyclass = "Mul")]
pub struct MulNode {
    pub children: Vec<NodeId>,
}

impl Node for MulNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|&id| values[id]).product()
    }
    
    fn as_serialized(&self) -> SerializedNode {
        SerializedNode::Mul { children: self.children.clone() }
    }
}

#[derive(Clone, SdagNode)]
#[sdag(pyclass = "Div")]
pub struct DivNode {
    pub left: NodeId,
    pub right: NodeId,
}

impl Node for DivNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        let l = values[self.left];
        let r = values[self.right];
        if r == 0.0 { f64::NAN } else { l / r }
    }
    
    fn as_serialized(&self) -> SerializedNode {
        SerializedNode::Div { left: self.left, right: self.right }
    }
}

// ===========================================================================
// GRAPH METHODS
// ===========================================================================

#[pymethods]
impl Graph {
    #[new]
    fn new() -> Self {
        Self::default()
    }
    
    fn freeze(&self, py: Python, root: PyObject) -> PyResult<String> {
        // Get root ID
        let root_id: usize = root.getattr(py, "id")?.extract(py)?;
        
        // Serialize nodes
        let serialized = SerializedGraph {
            nodes: self.arena.iter().map(|n| n.as_serialized()).collect(),
            root: root_id,
        };
        
        // Convert to YAML
        serde_yaml::to_string(&serialized)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
}

// ===========================================================================
// SAMPLER
// ===========================================================================

#[pyclass]
pub struct Sampler {
    nodes: Vec<Box<dyn Node>>,
    root: NodeId,
    outputs: Vec<NodeId>,
}

#[pymethods]
impl Sampler {
    #[new]
    fn new(graph_yaml: &str, outputs: Vec<NodeId>, _engine: Option<&str>) -> PyResult<Self> {
        // Deserialize graph
        let graph: SerializedGraph = serde_yaml::from_str(graph_yaml)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        // Build nodes
        let nodes: Vec<Box<dyn Node>> = graph.nodes.into_iter()
            .map(|n| -> Box<dyn Node> {
                match n {
                    SerializedNode::Input { name } => Box::new(InputNode { name }),
                    SerializedNode::Const { value } => Box::new(ConstNode { value }),
                    SerializedNode::Add { children } => Box::new(AddNode { children }),
                    SerializedNode::Mul { children } => Box::new(MulNode { children }),
                    SerializedNode::Div { left, right } => Box::new(DivNode { left, right }),
                }
            })
            .collect();
        
        Ok(Self {
            nodes,
            root: graph.root,
            outputs,
        })
    }
    
    fn run(&self, rows: Vec<HashMap<String, f64>>) -> PyResult<Vec<HashMap<String, f64>>> {
        let mut results = Vec::new();
        let mut prev_trigger: Option<f64> = None;
        
        for inputs in rows {
            // Evaluate all nodes in order (topological sort assumed)
            let mut values = vec![0.0; self.nodes.len()];
            for (i, node) in self.nodes.iter().enumerate() {
                values[i] = node.eval(&values, &inputs);
            }
            
            // Check trigger
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
    // The derive macro registers node classes automatically via inventory
    m.add_class::<Graph>()?;
    m.add_class::<Sampler>()?;
    Ok(())
}