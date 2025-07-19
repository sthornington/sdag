use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use crate::{Engine, engine};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global counter for generating unique node IDs
static NODE_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Base class for all DAG nodes in Python
#[pyclass(subclass)]
#[derive(Clone)]
struct Node {
    #[pyo3(get)]
    node_id: String,
    yaml_data: serde_yaml::Value,
}

#[pymethods]
impl Node {
    fn to_yaml(&self) -> PyResult<String> {
        serde_yaml::to_string(&self.yaml_data)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

/// Input node that receives streaming data
#[pyclass(extends=Node)]
struct InputNode {
    #[pyo3(get)]
    input_index: usize,
}

#[pymethods]
impl InputNode {
    #[new]
    fn new(input_index: usize) -> PyResult<(Self, Node)> {
        let node_id = format!("input_{}", NODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
        let yaml_data = json!({
            "id": &node_id,
            "type": "Input",
            "params": {
                "input_index": input_index
            }
        });
        
        Ok((
            InputNode { input_index },
            Node {
                node_id,
                yaml_data: serde_yaml::to_value(yaml_data).unwrap(),
            }
        ))
    }
}

/// Constant value node
#[pyclass(extends=Node)]
struct ConstantNode {
    #[pyo3(get)]
    value: f64,
}

#[pymethods]
impl ConstantNode {
    #[new]
    fn new(value: f64) -> PyResult<(Self, Node)> {
        let node_id = format!("const_{}", NODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
        let yaml_data = json!({
            "id": &node_id,
            "type": "Constant",
            "params": {
                "value": value
            }
        });
        
        Ok((
            ConstantNode { value },
            Node {
                node_id,
                yaml_data: serde_yaml::to_value(yaml_data).unwrap(),
            }
        ))
    }
}

/// Addition node
#[pyclass(extends=Node)]
struct AddNode {}

#[pymethods]
impl AddNode {
    #[new]
    fn new(_py: Python, a: &PyAny, b: &PyAny) -> PyResult<(Self, Node)> {
        let a_node: &PyCell<Node> = a.extract()?;
        let b_node: &PyCell<Node> = b.extract()?;
        
        let node_id = format!("add_{}", NODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
        let yaml_data = json!({
            "id": &node_id,
            "type": "Add",
            "params": {
                "inputs": [
                    a_node.borrow().node_id.clone(),
                    b_node.borrow().node_id.clone()
                ]
            }
        });
        
        Ok((
            AddNode {},
            Node {
                node_id,
                yaml_data: serde_yaml::to_value(yaml_data).unwrap(),
            }
        ))
    }
    
    // TODO: Add operator overloading support
}

/// Multiplication node
#[pyclass(extends=Node)]
struct MultiplyNode {}

#[pymethods]
impl MultiplyNode {
    #[new]
    fn new(_py: Python, a: &PyAny, b: &PyAny) -> PyResult<(Self, Node)> {
        let a_node: &PyCell<Node> = a.extract()?;
        let b_node: &PyCell<Node> = b.extract()?;
        
        let node_id = format!("mul_{}", NODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
        let yaml_data = json!({
            "id": &node_id,
            "type": "Multiply",
            "params": {
                "inputs": [
                    a_node.borrow().node_id.clone(),
                    b_node.borrow().node_id.clone()
                ]
            }
        });
        
        Ok((
            MultiplyNode {},
            Node {
                node_id,
                yaml_data: serde_yaml::to_value(yaml_data).unwrap(),
            }
        ))
    }
    
    // TODO: Add operator overloading support
}

/// Sum node for multiple inputs
#[pyclass(extends=Node)]
struct SumNode {}

#[pymethods]
impl SumNode {
    #[new]
    fn new(_py: Python, inputs: Vec<&PyAny>) -> PyResult<(Self, Node)> {
        let mut input_ids = Vec::new();
        
        for input in inputs {
            let node: &PyCell<Node> = input.extract()?;
            input_ids.push(node.borrow().node_id.clone());
        }
        
        let node_id = format!("sum_{}", NODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
        let yaml_data = json!({
            "id": &node_id,
            "type": "Sum",
            "params": {
                "inputs": input_ids
            }
        });
        
        Ok((
            SumNode {},
            Node {
                node_id,
                yaml_data: serde_yaml::to_value(yaml_data).unwrap(),
            }
        ))
    }
}

/// Comparison node
#[pyclass(extends=Node)]
struct ComparisonNode {
    #[pyo3(get)]
    op: String,
}

#[pymethods]
impl ComparisonNode {
    #[new]
    fn new(_py: Python, a: &PyAny, b: &PyAny, op: String) -> PyResult<(Self, Node)> {
        let a_node: &PyCell<Node> = a.extract()?;
        let b_node: &PyCell<Node> = b.extract()?;
        
        // Validate operation
        if !["GreaterThan", "LessThan", "Equal"].contains(&op.as_str()) {
            return Err(PyValueError::new_err(
                format!("Invalid comparison op: {}. Must be GreaterThan, LessThan, or Equal", op)
            ));
        }
        
        let node_id = format!("cmp_{}", NODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
        let yaml_data = json!({
            "id": &node_id,
            "type": "Comparison",
            "params": {
                "inputs": [
                    a_node.borrow().node_id.clone(),
                    b_node.borrow().node_id.clone()
                ],
                "op": &op
            }
        });
        
        Ok((
            ComparisonNode {
                op,
            },
            Node {
                node_id,
                yaml_data: serde_yaml::to_value(yaml_data).unwrap(),
            }
        ))
    }
}

// Comparison helper functions removed - use ComparisonNode directly

/// Graph class that collects nodes and can be converted to YAML
#[pyclass]
struct Graph {
    nodes: Vec<serde_yaml::Value>,
    trigger: Option<String>,
    outputs: Vec<String>,
}

#[pymethods]
impl Graph {
    #[new]
    fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            trigger: None,
            outputs: Vec::new(),
        }
    }
    
    /// Add a node to the graph
    fn add_node(&mut self, node: &PyAny) -> PyResult<()> {
        let node_obj: &PyCell<Node> = node.extract()?;
        let borrowed = node_obj.borrow();
        self.nodes.push(borrowed.yaml_data.clone());
        Ok(())
    }
    
    /// Set trigger node
    fn set_trigger(&mut self, node: &PyAny) -> PyResult<()> {
        let node_obj: &PyCell<Node> = node.extract()?;
        self.trigger = Some(node_obj.borrow().node_id.clone());
        Ok(())
    }
    
    /// Add output node
    fn add_output(&mut self, node: &PyAny) -> PyResult<()> {
        let node_obj: &PyCell<Node> = node.extract()?;
        self.outputs.push(node_obj.borrow().node_id.clone());
        Ok(())
    }
    
    /// Convert to YAML string
    fn to_yaml(&self) -> PyResult<String> {
        let dag = json!({
            "nodes": self.nodes,
            "trigger": self.trigger,
            "outputs": self.outputs
        });
        
        serde_yaml::to_string(&dag)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
    
    /// Build an engine from this graph
    fn build_engine(&self) -> PyResult<PyEngine> {
        let yaml = self.to_yaml()?;
        PyEngine::from_yaml(yaml)
    }
}

/// Python wrapper for the streaming DAG engine
#[pyclass]
struct PyEngine {
    engine: Engine,
}

#[pymethods]
impl PyEngine {
    /// Create a new engine from a YAML string
    #[staticmethod]
    fn from_yaml(yaml_str: String) -> PyResult<Self> {
        let engine = engine::from_yaml(&yaml_str)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(PyEngine { engine })
    }
    
    /// Load a DAG from a YAML file
    #[staticmethod]
    fn from_yaml_file(path: String) -> PyResult<Self> {
        let yaml_str = std::fs::read_to_string(&path)
            .map_err(|e| PyValueError::new_err(format!("Failed to read file: {}", e)))?;
        Self::from_yaml(yaml_str)
    }
    
    /// Create engine from a Graph object
    #[staticmethod]
    fn from_graph(graph: &Graph) -> PyResult<Self> {
        let yaml = graph.to_yaml()?;
        Self::from_yaml(yaml)
    }
    
    /// Evaluate one step with input values
    /// Returns None if trigger didn't fire, or a list of output values if it did
    fn evaluate_step(&mut self, input_values: Vec<f64>) -> PyResult<Option<Vec<f64>>> {
        Ok(self.engine.evaluate_step(&input_values))
    }
    
    /// Get the current value of a specific node
    fn get_value(&self, node_id: usize) -> PyResult<f64> {
        if node_id >= self.engine.get_all_values().len() {
            return Err(PyValueError::new_err(format!("Node {} does not exist", node_id)));
        }
        Ok(self.engine.get_value(node_id))
    }
    
    /// Get all current node values
    fn get_all_values(&self) -> PyResult<Vec<f64>> {
        Ok(self.engine.get_all_values().to_vec())
    }
    
    /// Process a stream of input rows and yield outputs when trigger fires
    fn stream(&mut self, py: Python, input_stream: &PyAny) -> PyResult<Vec<Vec<f64>>> {
        let mut outputs = Vec::new();
        
        // Iterate through the input stream
        for row in input_stream.iter()? {
            let input_values: Vec<f64> = row?.extract()?;
            
            // Process this row
            if let Some(output_values) = self.engine.evaluate_step(&input_values) {
                outputs.push(output_values);
            }
            
            // Allow other Python threads to run
            py.check_signals()?;
        }
        
        Ok(outputs)
    }
}

#[pymodule]
fn sdag(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Node>()?;
    m.add_class::<InputNode>()?;
    m.add_class::<ConstantNode>()?;
    m.add_class::<AddNode>()?;
    m.add_class::<MultiplyNode>()?;
    m.add_class::<SumNode>()?;
    m.add_class::<ComparisonNode>()?;
    m.add_class::<Graph>()?;
    m.add_class::<PyEngine>()?;
    Ok(())
}