use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use crate::{Engine, engine};
use serde_json::json;

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

/// Python wrapper for building DAGs programmatically
#[pyclass]
struct PyDagBuilder {
    yaml_nodes: Vec<serde_yaml::Value>,
    trigger: Option<String>,
    outputs: Vec<String>,
}

#[pymethods]
impl PyDagBuilder {
    #[new]
    fn new() -> Self {
        PyDagBuilder {
            yaml_nodes: Vec::new(),
            trigger: None,
            outputs: Vec::new(),
        }
    }
    
    /// Add a constant node
    fn add_constant(&mut self, id: String, value: f64) -> PyResult<()> {
        let node = serde_yaml::to_value(json!({
            "id": id,
            "type": "Constant",
            "params": {
                "value": value
            }
        })).map_err(|e| PyValueError::new_err(e.to_string()))?;
        
        self.yaml_nodes.push(node);
        Ok(())
    }
    
    /// Add an input node
    fn add_input(&mut self, id: String, input_index: usize) -> PyResult<()> {
        let node = serde_yaml::to_value(json!({
            "id": id,
            "type": "Input",
            "params": {
                "input_index": input_index
            }
        })).map_err(|e| PyValueError::new_err(e.to_string()))?;
        
        self.yaml_nodes.push(node);
        Ok(())
    }
    
    /// Add an addition node
    fn add_add(&mut self, id: String, input_a: String, input_b: String) -> PyResult<()> {
        let node = serde_yaml::to_value(json!({
            "id": id,
            "type": "Add",
            "params": {
                "inputs": [input_a, input_b]
            }
        })).map_err(|e| PyValueError::new_err(e.to_string()))?;
        
        self.yaml_nodes.push(node);
        Ok(())
    }
    
    /// Add a multiplication node
    fn add_multiply(&mut self, id: String, input_a: String, input_b: String) -> PyResult<()> {
        let node = serde_yaml::to_value(json!({
            "id": id,
            "type": "Multiply",
            "params": {
                "inputs": [input_a, input_b]
            }
        })).map_err(|e| PyValueError::new_err(e.to_string()))?;
        
        self.yaml_nodes.push(node);
        Ok(())
    }
    
    /// Add a comparison node
    fn add_comparison(&mut self, id: String, input_a: String, input_b: String, op: String) -> PyResult<()> {
        // Validate operation
        if !["GreaterThan", "LessThan", "Equal"].contains(&op.as_str()) {
            return Err(PyValueError::new_err(format!("Invalid comparison op: {}. Must be GreaterThan, LessThan, or Equal", op)));
        }
        
        let node = serde_yaml::to_value(json!({
            "id": id,
            "type": "Comparison",
            "params": {
                "inputs": [input_a, input_b],
                "op": op
            }
        })).map_err(|e| PyValueError::new_err(e.to_string()))?;
        
        self.yaml_nodes.push(node);
        Ok(())
    }
    
    /// Set the trigger node
    fn set_trigger(&mut self, node_id: String) -> PyResult<()> {
        self.trigger = Some(node_id);
        Ok(())
    }
    
    /// Set the output nodes
    fn set_outputs(&mut self, node_ids: Vec<String>) -> PyResult<()> {
        self.outputs = node_ids;
        Ok(())
    }
    
    /// Build the engine from the current DAG definition
    fn build(&self) -> PyResult<PyEngine> {
        // Construct the YAML structure
        let dag = json!({
            "nodes": self.yaml_nodes,
            "trigger": self.trigger,
            "outputs": self.outputs
        });
        
        let yaml_str = serde_yaml::to_string(&dag)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        
        PyEngine::from_yaml(yaml_str)
    }
    
    /// Get the YAML representation
    fn to_yaml(&self) -> PyResult<String> {
        let dag = json!({
            "nodes": self.yaml_nodes,
            "trigger": self.trigger,
            "outputs": self.outputs
        });
        
        serde_yaml::to_string(&dag)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

/// Run a streaming DAG from a YAML file with command line arguments
#[pyfunction]
fn run_dag_cli(args: Vec<String>) -> PyResult<()> {
    if args.len() < 1 {
        return Err(PyValueError::new_err("Usage: run_dag_cli(['dag.yaml', '1.0', '2.0', ...])"));
    }
    
    let yaml_content = std::fs::read_to_string(&args[0])
        .map_err(|e| PyValueError::new_err(format!("Failed to read file: {}", e)))?;
    
    let mut engine = engine::from_yaml(&yaml_content)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    
    // Parse input values
    let input_values: Vec<f64> = if args.len() > 1 {
        args[1..].iter()
            .map(|s| s.parse::<f64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| PyValueError::new_err(format!("Failed to parse input value: {}", e)))?
    } else {
        vec![0.0; 10] // Default to zeros
    };
    
    println!("Loaded DAG from {}", args[0]);
    println!("Evaluating with inputs: {:?}", input_values);
    
    // Run one evaluation step
    if let Some(outputs) = engine.evaluate_step(&input_values) {
        println!("\nTrigger fired! Outputs: {:?}", outputs);
    } else {
        println!("\nNo trigger fired");
    }
    
    // Show all node values
    println!("\nAll node values:");
    for (i, val) in engine.get_all_values().iter().enumerate() {
        println!("  Node {}: {}", i, val);
    }
    
    Ok(())
}

#[pymodule]
fn sdag(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyEngine>()?;
    m.add_class::<PyDagBuilder>()?;
    m.add_function(wrap_pyfunction!(run_dag_cli, m)?)?;
    Ok(())
}