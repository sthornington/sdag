use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3::types::{PyDict, PyTuple};
use crate::{Engine, engine};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;

/// Global counter for generating unique node IDs
static NODE_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Graph class that collects nodes
#[pyclass]
#[derive(Clone)]
pub struct Graph {
    nodes: Vec<serde_yaml::Value>,
    node_ids: Vec<String>,
    trigger: Option<String>,
    outputs: Vec<String>,
}

#[pymethods]
impl Graph {
    #[new]
    fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            node_ids: Vec::new(),
            trigger: None,
            outputs: Vec::new(),
        }
    }
    
    fn to_yaml(&self) -> PyResult<String> {
        let dag = json!({
            "nodes": self.nodes,
            "trigger": self.trigger,
            "outputs": self.outputs
        });
        
        serde_yaml::to_string(&dag)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
    
    fn build_engine(&self) -> PyResult<PyEngine> {
        let yaml = self.to_yaml()?;
        PyEngine::from_yaml(yaml)
    }
}

impl Graph {
    fn add_node(&mut self, node_id: String, yaml_data: serde_yaml::Value) -> PyResult<()> {
        if !self.node_ids.contains(&node_id) {
            self.nodes.push(yaml_data);
            self.node_ids.push(node_id);
        }
        Ok(())
    }
    
    pub fn set_trigger(&mut self, node_id: String) -> PyResult<()> {
        self.trigger = Some(node_id);
        Ok(())
    }
    
    pub fn add_output(&mut self, node_id: String) -> PyResult<()> {
        if !self.outputs.contains(&node_id) {
            self.outputs.push(node_id);
        }
        Ok(())
    }
}

/// Base trait for all nodes
trait PyNode {
    fn node_id(&self) -> &str;
    fn graph(&self) -> &Py<Graph>;
}

/// Special case: Input node (leaf node that references external data)
#[pyclass]
#[derive(Clone)]
pub struct Input {
    node_id: String,
    #[pyo3(get)]
    graph: Py<Graph>,
}

#[pymethods]
impl Input {
    #[new]
    fn new(py: Python, graph: Py<Graph>, input_index: usize) -> PyResult<Self> {
        let node_id = format!("input_{}", NODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
        
        let yaml_data = serde_yaml::to_value(json!({
            "id": &node_id,
            "type": "Input",
            "params": {
                "input_index": input_index
            }
        })).unwrap();
        
        graph.borrow_mut(py).add_node(node_id.clone(), yaml_data)?;
        
        Ok(Self {
            node_id,
            graph: graph.clone(),
        })
    }
    
    #[getter]
    fn node_id(&self) -> &str {
        &self.node_id
    }
    
    fn output(&self, py: Python) -> PyResult<Self> {
        self.graph.borrow_mut(py).add_output(self.node_id.clone())?;
        Ok(self.clone())
    }
    
    fn trigger(&self, py: Python) -> PyResult<Self> {
        self.graph.borrow_mut(py).set_trigger(self.node_id.clone())?;
        Ok(self.clone())
    }
}

/// Special case: Constant node (leaf node with a fixed value)
#[pyclass]
#[derive(Clone)]
pub struct Constant {
    node_id: String,
    #[pyo3(get)]
    graph: Py<Graph>,
}

#[pymethods]
impl Constant {
    #[new]
    fn new(py: Python, graph: Py<Graph>, value: f64) -> PyResult<Self> {
        let node_id = format!("constant_{}", NODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
        
        let yaml_data = serde_yaml::to_value(json!({
            "id": &node_id,
            "type": "Constant",
            "params": {
                "value": value
            }
        })).unwrap();
        
        graph.borrow_mut(py).add_node(node_id.clone(), yaml_data)?;
        
        Ok(Self {
            node_id,
            graph: graph.clone(),
        })
    }
    
    #[getter]
    fn node_id(&self) -> &str {
        &self.node_id
    }
    
    fn output(&self, py: Python) -> PyResult<Self> {
        self.graph.borrow_mut(py).add_output(self.node_id.clone())?;
        Ok(self.clone())
    }
    
    fn trigger(&self, py: Python) -> PyResult<Self> {
        self.graph.borrow_mut(py).set_trigger(self.node_id.clone())?;
        Ok(self.clone())
    }
}

/// Generic macro for transform nodes (nodes that take other nodes as inputs)
/// These all follow the same pattern: collect node inputs into "inputs" array
/// and any additional parameters as kwargs
macro_rules! transform_node {
    ($name:ident) => {
        #[pyclass]
        #[derive(Clone)]
        pub struct $name {
            node_id: String,
            #[pyo3(get)]
            graph: Py<Graph>,
        }
        
        impl PyNode for $name {
            fn node_id(&self) -> &str {
                &self.node_id
            }
            
            fn graph(&self) -> &Py<Graph> {
                &self.graph
            }
        }
        
        #[pymethods]
        impl $name {
            #[new]
            #[pyo3(signature = (graph, *args, **kwargs))]
            fn new(py: Python, graph: Py<Graph>, args: &PyTuple, kwargs: Option<&PyDict>) -> PyResult<Self> {
                let node_id = format!("{}_{}", 
                    stringify!($name).to_lowercase(), 
                    NODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
                );
                
                let mut params = HashMap::new();
                
                // All positional args are node inputs
                let mut inputs = Vec::new();
                for arg in args.iter() {
                    if let Ok(node_id) = arg.getattr("node_id") {
                        inputs.push(node_id.extract::<String>()?);
                    } else if let Ok(nodes) = arg.extract::<Vec<&PyAny>>() {
                        // Handle list of nodes (for Sum etc)
                        for node in nodes {
                            if let Ok(node_id) = node.getattr("node_id") {
                                inputs.push(node_id.extract::<String>()?);
                            }
                        }
                    }
                }
                
                if !inputs.is_empty() {
                    params.insert("inputs".to_string(), json!(inputs));
                }
                
                // All kwargs become additional parameters
                if let Some(kwargs) = kwargs {
                    for (key, value) in kwargs.iter() {
                        let key_str: String = key.extract()?;
                        if let Ok(val) = value.extract::<f64>() {
                            params.insert(key_str, json!(val));
                        } else if let Ok(val) = value.extract::<String>() {
                            params.insert(key_str, json!(val));
                        } else if let Ok(val) = value.extract::<i64>() {
                            params.insert(key_str, json!(val));
                        }
                    }
                }
                
                let yaml_data = serde_yaml::to_value(json!({
                    "id": &node_id,
                    "type": stringify!($name),
                    "params": params
                })).unwrap();
                
                graph.borrow_mut(py).add_node(node_id.clone(), yaml_data)?;
                
                Ok(Self {
                    node_id,
                    graph: graph.clone(),
                })
            }
            
            #[getter]
            fn node_id(&self) -> &str {
                &self.node_id
            }
            
            fn output(&self, py: Python) -> PyResult<Self> {
                self.graph.borrow_mut(py).add_output(self.node_id.clone())?;
                Ok(self.clone())
            }
            
            fn trigger(&self, py: Python) -> PyResult<Self> {
                self.graph.borrow_mut(py).set_trigger(self.node_id.clone())?;
                Ok(self.clone())
            }
        }
    };
}

// Generate transform node classes - these are all generic!
transform_node!(Add);
transform_node!(Multiply);
transform_node!(Sum);
transform_node!(ConstantProduct);
transform_node!(Comparison);
transform_node!(Pow);

/// Python wrapper for the streaming DAG engine
#[pyclass]
struct PyEngine {
    engine: Engine,
}

#[pymethods]
impl PyEngine {
    #[staticmethod]
    fn from_yaml(yaml_str: String) -> PyResult<Self> {
        let engine = engine::from_yaml(&yaml_str)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(PyEngine { engine })
    }
    
    #[staticmethod]
    fn from_yaml_file(path: String) -> PyResult<Self> {
        let yaml_str = std::fs::read_to_string(&path)
            .map_err(|e| PyValueError::new_err(format!("Failed to read file: {}", e)))?;
        Self::from_yaml(yaml_str)
    }
    
    #[staticmethod]
    fn from_graph(graph: &Graph) -> PyResult<Self> {
        graph.build_engine()
    }
    
    fn evaluate_step(&mut self, input_values: Vec<f64>) -> PyResult<Option<Vec<f64>>> {
        Ok(self.engine.evaluate_step(&input_values))
    }
    
    fn get_value(&self, node_id: usize) -> PyResult<f64> {
        if node_id >= self.engine.get_all_values().len() {
            return Err(PyValueError::new_err(format!("Node {} does not exist", node_id)));
        }
        Ok(self.engine.get_value(node_id))
    }
    
    fn get_all_values(&self) -> PyResult<Vec<f64>> {
        Ok(self.engine.get_all_values().to_vec())
    }
    
    fn stream(&mut self, py: Python, input_stream: &PyAny) -> PyResult<Vec<Vec<f64>>> {
        let mut outputs = Vec::new();
        
        for row in input_stream.iter()? {
            let input_values: Vec<f64> = row?.extract()?;
            
            if let Some(output_values) = self.engine.evaluate_step(&input_values) {
                outputs.push(output_values);
            }
            
            py.check_signals()?;
        }
        
        Ok(outputs)
    }
}

#[pymodule]
fn sdag(_py: Python, m: &PyModule) -> PyResult<()> {
    // Core classes
    m.add_class::<Graph>()?;
    m.add_class::<PyEngine>()?;
    
    // Special leaf nodes
    m.add_class::<Input>()?;
    m.add_class::<Constant>()?;
    
    // Transform nodes (all generic!)
    m.add_class::<Add>()?;
    m.add_class::<Multiply>()?;
    m.add_class::<Sum>()?;
    m.add_class::<ConstantProduct>()?;
    m.add_class::<Comparison>()?;
    m.add_class::<Pow>()?;
    
    Ok(())
}