use pyo3::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ===========================================================================
// CORE TYPES
// ===========================================================================

pub type NodeId = usize;

// The Python-visible graph builder
#[pyclass]
pub struct Graph {
    next_id: NodeId,
    registry: HashMap<String, PyObject>,  // id -> python node object
}

// The evaluation graph - pure Rust, built from YAML
pub struct EvalGraph {
    nodes: Vec<Node>,
    root: NodeId,
}

// All node types in one enum for simplicity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Node {
    Input { name: String },
    Const { value: f64 },
    Add { children: Vec<NodeId> },
    Mul { children: Vec<NodeId> },
    Div { left: NodeId, right: NodeId },
}

impl Node {
    fn eval(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        match self {
            Node::Input { name } => *inputs.get(name).unwrap_or(&0.0),
            Node::Const { value } => *value,
            Node::Add { children } => children.iter().map(|&id| values[id]).sum(),
            Node::Mul { children } => children.iter().map(|&id| values[id]).product(),
            Node::Div { left, right } => {
                let l = values[*left];
                let r = values[*right];
                if r == 0.0 { f64::NAN } else { l / r }
            }
        }
    }
}

// Serializable graph format
#[derive(Serialize, Deserialize)]
struct SerializedGraph {
    nodes: Vec<Node>,
    root: NodeId,
}

// ===========================================================================
// PYTHON NODE CLASSES
// ===========================================================================

macro_rules! pynode {
    ($name:ident { $($field:ident: $ftype:ty => $ptype:ty),* }) => {
        #[pyclass]
        #[derive(Clone)]
        pub struct $name {
            #[pyo3(get)]
            pub id: String,
            $(
                #[pyo3(get)]
                pub $field: $ptype,
            )*
        }
        
        #[pymethods]
        impl $name {
            #[new]
            fn new(graph: &mut Graph, $($field: $ptype),*) -> Self {
                let id = format!("n{}", graph.next_id);
                graph.next_id += 1;
                
                let node = Self { id: id.clone(), $($field),* };
                graph.registry.insert(id.clone(), node.clone().into_py(graph.py()));
                node
            }
        }
    };
}

pynode!(Input { name: String => String });
pynode!(Const { value: f64 => f64 });
pynode!(Add { children: Vec<NodeId> => Vec<PyObject> });
pynode!(Mul { children: Vec<NodeId> => Vec<PyObject> });
pynode!(Div { left: NodeId => PyObject, right: NodeId => PyObject });

// ===========================================================================
// GRAPH IMPLEMENTATION
// ===========================================================================

#[pymethods]
impl Graph {
    #[new]
    fn new() -> Self {
        Self {
            next_id: 0,
            registry: HashMap::new(),
        }
    }
    
    fn freeze(&self, py: Python, root: PyObject) -> PyResult<String> {
        // Get root ID
        let root_id: String = root.getattr(py, "id")?.extract(py)?;
        
        // Traverse graph to find all reachable nodes
        let mut seen = Vec::new();
        let mut stack = vec![root];
        
        while let Some(obj) = stack.pop() {
            let id: String = obj.getattr(py, "id")?.extract(py)?;
            if seen.contains(&id) { continue; }
            seen.push(id);
            
            // Check node type and traverse children
            if let Ok(children) = obj.getattr(py, "children") {
                if let Ok(children_vec) = children.extract::<Vec<PyObject>>(py) {
                    stack.extend(children_vec);
                }
            }
            if let Ok(left) = obj.getattr(py, "left") {
                stack.push(left);
            }
            if let Ok(right) = obj.getattr(py, "right") {
                stack.push(right);
            }
        }
        
        // Build nodes in topological order
        seen.reverse();
        let mut id_map: HashMap<String, NodeId> = HashMap::new();
        let mut nodes = Vec::new();
        
        for (idx, id) in seen.iter().enumerate() {
            id_map.insert(id.clone(), idx);
            
            let obj = &self.registry[id];
            let node_type = obj.as_ref(py).get_type().name()?;
            
            let node = match node_type {
                "Input" => {
                    let name: String = obj.getattr(py, "name")?.extract(py)?;
                    Node::Input { name }
                },
                "Const" => {
                    let value: f64 = obj.getattr(py, "value")?.extract(py)?;
                    Node::Const { value }
                },
                "Add" => {
                    let children: Vec<PyObject> = obj.getattr(py, "children")?.extract(py)?;
                    let children = children.into_iter()
                        .map(|c| -> PyResult<NodeId> {
                            let cid: String = c.getattr(py, "id")?.extract(py)?;
                            Ok(id_map[&cid])
                        })
                        .collect::<PyResult<Vec<_>>>()?;
                    Node::Add { children }
                },
                "Mul" => {
                    let children: Vec<PyObject> = obj.getattr(py, "children")?.extract(py)?;
                    let children = children.into_iter()
                        .map(|c| -> PyResult<NodeId> {
                            let cid: String = c.getattr(py, "id")?.extract(py)?;
                            Ok(id_map[&cid])
                        })
                        .collect::<PyResult<Vec<_>>>()?;
                    Node::Mul { children }
                },
                "Div" => {
                    let left: PyObject = obj.getattr(py, "left")?.extract(py)?;
                    let right: PyObject = obj.getattr(py, "right")?.extract(py)?;
                    let left_id: String = left.getattr(py, "id")?.extract(py)?;
                    let right_id: String = right.getattr(py, "id")?.extract(py)?;
                    Node::Div { 
                        left: id_map[&left_id], 
                        right: id_map[&right_id] 
                    }
                },
                _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    format!("Unknown node type: {}", node_type)
                )),
            };
            
            nodes.push(node);
        }
        
        let graph = SerializedGraph {
            nodes,
            root: id_map[&root_id],
        };
        
        serde_yaml::to_string(&graph)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
}

// ===========================================================================
// SAMPLER
// ===========================================================================

#[pyclass]
pub struct Sampler {
    graph: EvalGraph,
    outputs: Vec<NodeId>,
}

#[pymethods]
impl Sampler {
    #[new]
    fn new(yaml: &str, outputs: Vec<NodeId>, _engine: Option<&str>) -> PyResult<Self> {
        let sg: SerializedGraph = serde_yaml::from_str(yaml)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        Ok(Self {
            graph: EvalGraph {
                nodes: sg.nodes,
                root: sg.root,
            },
            outputs,
        })
    }
    
    fn run(&self, rows: Vec<HashMap<String, f64>>) -> PyResult<Vec<HashMap<String, f64>>> {
        let mut results = Vec::new();
        let mut prev_trigger: Option<f64> = None;
        
        for inputs in rows {
            // Flat sweep evaluation - assumes topological order
            let mut values = vec![0.0; self.graph.nodes.len()];
            for (i, node) in self.graph.nodes.iter().enumerate() {
                values[i] = node.eval(&values, &inputs);
            }
            
            // Trigger-based output
            let trigger = values[self.graph.root];
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
fn sdag(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Graph>()?;
    m.add_class::<Sampler>()?;
    
    // Add node classes
    m.add_class::<Input>()?;
    m.add_class::<Const>()?;
    m.add_class::<Add>()?;
    m.add_class::<Mul>()?;
    m.add_class::<Div>()?;
    
    // Add convenience factory functions to Graph
    m.add_function(wrap_pyfunction!(input, m)?)?;
    m.add_function(wrap_pyfunction!(const_, m)?)?;
    m.add_function(wrap_pyfunction!(add, m)?)?;
    m.add_function(wrap_pyfunction!(mul, m)?)?;
    m.add_function(wrap_pyfunction!(div, m)?)?;
    
    Ok(())
}

// Convenience functions that mirror the current API
#[pyfunction]
fn input(graph: &mut Graph, name: String) -> Input {
    Input::new(graph, name)
}

#[pyfunction]
#[pyo3(name = "const")]
fn const_(graph: &mut Graph, value: f64) -> Const {
    Const::new(graph, value)
}

#[pyfunction]
fn add(graph: &mut Graph, children: Vec<PyObject>) -> Add {
    Add::new(graph, children)
}

#[pyfunction]
fn mul(graph: &mut Graph, children: Vec<PyObject>) -> Mul {
    Mul::new(graph, children)
}

#[pyfunction]
fn div(graph: &mut Graph, left: PyObject, right: PyObject) -> Div {
    Div::new(graph, left, right)
}