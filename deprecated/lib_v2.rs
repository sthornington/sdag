#[macro_use]
extern crate inventory;

use pyo3::prelude::*;
use pyo3::types::PySequence;
use std::collections::HashMap;

#[macro_use]
mod node_macro_v2;
mod arena;
mod engine_traits;
mod engines;
mod nodes_v2;

use arena::{Arena, ArenaGraph, ArenaNode, NodeId};
use engine_traits::{Engine, NodeRegistry};
use engines::{LazyEngine, TopologicalEngine};

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
        self.create_node(py, "input", vec![("name", name.into_py(py))])
    }
    
    #[pyo3(name = "r#const")]
    fn const_node(&mut self, py: Python, value: f64) -> PyObject {
        self.create_node(py, "const", vec![("value", value.into_py(py))])
    }
    
    fn add(&mut self, py: Python, children: Vec<PyObject>) -> PyObject {
        self.create_node(py, "add", vec![("children", children.into_py(py))])
    }
    
    fn mul(&mut self, py: Python, children: Vec<PyObject>) -> PyObject {
        self.create_node(py, "mul", vec![("children", children.into_py(py))])
    }
    
    fn div(&mut self, py: Python, left: PyObject, right: PyObject) -> PyObject {
        self.create_node(py, "div", vec![("left", left), ("right", right)])
    }
    
    fn freeze(&self, py: Python, root: PyObject) -> PyResult<String> {
        // Build arena graph from Python objects
        let mut arena = Arena::<ArenaNode>::new();
        let mut py_to_arena = HashMap::new();
        
        // Discover all reachable nodes
        let root_id: String = root.as_ref(py).getattr("id")?.extract()?;
        let mut stack = vec![root.clone()];
        let mut seen = std::collections::HashSet::new();
        
        while let Some(node) = stack.pop() {
            let id: String = node.as_ref(py).getattr("id")?.extract()?;
            if !seen.insert(id.clone()) {
                continue;
            }
            
            // Get node type and fields
            let node_type: String = node.as_ref(py).get_type().getattr("TYPE")?.extract()?;
            let fields: Vec<String> = node.as_ref(py).get_type().getattr("FIELDS")?.extract()?;
            
            // Build data map
            let mut data = serde_yaml::Mapping::new();
            
            for field in fields {
                let value = node.as_ref(py).getattr(field.as_str())?;
                
                let yaml_value = if let Ok(children) = value.cast_as::<PySequence>() {
                    // Handle Vec<NodeId>
                    let mut child_ids = Vec::new();
                    for child in children.iter()? {
                        let child_obj: PyObject = child?.extract()?;
                        let child_id: String = child_obj.as_ref(py).getattr("id")?.extract()?;
                        child_ids.push(serde_yaml::Value::String(child_id.clone()));
                        stack.push(child_obj);
                    }
                    serde_yaml::Value::Sequence(child_ids)
                } else if let Ok(child) = value.extract::<PyObject>() {
                    if child.as_ref(py).hasattr("id")? {
                        // Handle single NodeId
                        let child_id: String = child.as_ref(py).getattr("id")?.extract()?;
                        stack.push(child);
                        serde_yaml::Value::String(child_id)
                    } else {
                        // Handle other types
                        if let Ok(s) = value.extract::<String>() {
                            serde_yaml::Value::String(s)
                        } else if let Ok(f) = value.extract::<f64>() {
                            serde_yaml::to_value(f).unwrap()
                        } else {
                            continue;
                        }
                    }
                } else {
                    continue;
                };
                
                data.insert(serde_yaml::Value::String(field), yaml_value);
            }
            
            // Insert into arena
            let arena_id = arena.insert(
                ArenaNode {
                    id: 0, // Will be updated
                    node_type,
                    data: serde_yaml::Value::Mapping(data),
                },
                Some(id.clone()),
            );
            py_to_arena.insert(id, arena_id);
        }
        
        // Update node IDs and references
        let nodes: Vec<ArenaNode> = arena.nodes().iter().enumerate().map(|(i, node)| {
            let mut updated = node.clone();
            updated.id = i;
            
            // Update references in data
            if let serde_yaml::Value::Mapping(ref mut map) = updated.data {
                for (_key, value) in map.iter_mut() {
                    match value {
                        serde_yaml::Value::String(ref mut s) => {
                            if let Some(&arena_id) = py_to_arena.get(s) {
                                *value = serde_yaml::Value::Number(arena_id.into());
                            }
                        }
                        serde_yaml::Value::Sequence(ref mut seq) => {
                            for item in seq.iter_mut() {
                                if let serde_yaml::Value::String(ref s) = item {
                                    if let Some(&arena_id) = py_to_arena.get(s) {
                                        *item = serde_yaml::Value::Number(arena_id.into());
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            
            updated
        }).collect();
        
        let root_arena_id = py_to_arena[&root_id];
        
        let graph = ArenaGraph {
            nodes,
            root: root_arena_id,
        };
        
        graph.to_yaml().map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
    }
    
    // Helper to create nodes
    fn create_node(&mut self, py: Python, node_type: &str, fields: Vec<(&str, PyObject)>) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        
        // Create a dynamic Python object
        let node_class = match node_type {
            "input" => py.get_type::<nodes_v2::InputNodePy>(),
            "const" => py.get_type::<nodes_v2::ConstNodePy>(),
            "add" => py.get_type::<nodes_v2::AddNodePy>(),
            "mul" => py.get_type::<nodes_v2::MulNodePy>(),
            "div" => py.get_type::<nodes_v2::DivNodePy>(),
            _ => panic!("Unknown node type"),
        };
        
        let mut args = vec![id.clone().into_py(py)];
        for (_, value) in fields {
            args.push(value);
        }
        
        let node = node_class.call1(pyo3::types::PyTuple::new(py, args)).unwrap();
        let py_node: PyObject = node.into();
        
        self.registry.insert(id, py_node.clone());
        py_node
    }
}

/// Python Sampler
#[pyclass]
struct Sampler {
    graph_yaml: String,
    outputs: Vec<NodeId>,
    engine_name: String,
}

#[pymethods]
impl Sampler {
    #[new]
    #[pyo3(signature = (graph, outputs, engine = "topological"))]
    fn new(graph: &str, outputs: Vec<usize>, engine: &str) -> PyResult<Self> {
        // Validate graph
        ArenaGraph::from_yaml(graph)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
        
        Ok(Sampler {
            graph_yaml: graph.to_string(),
            outputs,
            engine_name: engine.to_string(),
        })
    }
    
    fn run(&self, rows: Vec<HashMap<String, f64>>) -> PyResult<Vec<HashMap<String, f64>>> {
        // Parse graph
        let arena_graph = ArenaGraph::from_yaml(&self.graph_yaml)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
        
        // Build nodes
        let mut registry = NodeRegistry::new();
        nodes_v2::register_all_nodes(&mut registry);
        
        let mut nodes = Vec::new();
        for arena_node in &arena_graph.nodes {
            let node = registry.build(arena_node)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))?;
            nodes.push(node);
        }
        
        // Select engine
        let engine: Box<dyn Engine> = match self.engine_name.as_str() {
            "topological" => Box::new(TopologicalEngine),
            "lazy" => Box::new(LazyEngine),
            _ => return Err(pyo3::exceptions::PyValueError::new_err(
                format!("Unknown engine: {}", self.engine_name)
            )),
        };
        
        // Run evaluation
        Ok(engine.evaluate(&nodes, arena_graph.root, &self.outputs, rows))
    }
}

/// Python module
#[pymodule]
fn sdag(py: Python, m: &PyModule) -> PyResult<()> {
    nodes_v2::register_all_python(m)?;
    m.add_class::<Graph>()?;
    m.add_class::<Sampler>()?;
    Ok(())
}