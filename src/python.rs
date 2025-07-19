use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3::types::{PyDict, PyList};
use crate::{NodeRegistry, Value, DagYaml, yaml::{ConnectionYaml, NodeYaml}};

#[pyclass]
struct PyDag {
    nodes: Arc<Mutex<Vec<NodeYaml>>>,
    connections: Arc<Mutex<Vec<ConnectionYaml>>>,
    registry: Arc<NodeRegistry>,
}

#[pymethods]
impl PyDag {
    #[new]
    fn new() -> Self {
        PyDag { 
            nodes: Arc::new(Mutex::new(Vec::new())),
            connections: Arc::new(Mutex::new(Vec::new())),
            registry: Arc::new(NodeRegistry::new()),
        }
    }

    fn add_node(&self, id: String, node_type: String, params: Option<&PyDict>) -> PyResult<()> {
        let rust_params = if let Some(params) = params {
            py_dict_to_value_map(params)?
        } else {
            HashMap::new()
        };

        self.nodes.lock().unwrap().push(NodeYaml {
            id,
            node_type,
            params: rust_params,
        });
        
        Ok(())
    }

    fn connect(&self, from_node: String, from_output: String, to_node: String, to_input: String) -> PyResult<()> {
        self.connections.lock().unwrap().push(ConnectionYaml {
            from_node,
            from_output,
            to_node,
            to_input,
        });
        
        Ok(())
    }

    fn execute(&self, py: Python) -> PyResult<PyObject> {
        let dag_yaml = self.build_dag_yaml()?;
        let dag = dag_yaml.to_dag(self.registry.clone())
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        let results = dag.execute()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        let py_results = PyDict::new(py);
        for (node_id, outputs) in results {
            let py_outputs = value_map_to_py_dict(py, &outputs)?;
            py_results.set_item(node_id, py_outputs)?;
        }

        Ok(py_results.into())
    }

    fn to_yaml(&self) -> PyResult<String> {
        let dag_yaml = self.build_dag_yaml()?;
        dag_yaml.to_yaml()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn save_yaml(&self, path: String) -> PyResult<()> {
        let yaml_str = self.to_yaml()?;
        std::fs::write(path, yaml_str)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
}

impl PyDag {
    fn build_dag_yaml(&self) -> PyResult<DagYaml> {
        let nodes = self.nodes.lock().unwrap().clone();
        let connections = self.connections.lock().unwrap().clone();
        Ok(DagYaml { nodes, connections })
    }
}

#[pyfunction]
fn load_dag_from_yaml(yaml_str: String) -> PyResult<PyDag> {
    let dag_yaml = DagYaml::from_yaml(&yaml_str)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    
    let py_dag = PyDag::new();
    
    // Add nodes
    Python::with_gil(|py| -> PyResult<()> {
        for node in &dag_yaml.nodes {
            let dict = PyDict::new(py);
            for (k, v) in &node.params {
                dict.set_item(k, value_to_py(py, v)?)?;
            }
            py_dag.add_node(node.id.clone(), node.node_type.clone(), Some(dict))?;
        }
        Ok(())
    })?;
    
    // Add connections
    for conn in &dag_yaml.connections {
        py_dag.connect(
            conn.from_node.clone(),
            conn.from_output.clone(),
            conn.to_node.clone(),
            conn.to_input.clone()
        )?;
    }
    
    Ok(py_dag)
}

fn py_dict_to_value_map(dict: &PyDict) -> PyResult<HashMap<String, Value>> {
    let mut map = HashMap::new();
    
    for (key, value) in dict {
        let key_str = key.extract::<String>()?;
        let rust_value = py_to_value(value)?;
        map.insert(key_str, rust_value);
    }
    
    Ok(map)
}

fn py_to_value(obj: &PyAny) -> PyResult<Value> {
    if obj.is_none() {
        Ok(Value::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(Value::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(Value::Integer(i))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(Value::Float(f))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(Value::String(s))
    } else if let Ok(list) = obj.downcast::<PyList>() {
        let mut vec = Vec::new();
        for item in list {
            vec.push(py_to_value(item)?);
        }
        Ok(Value::Array(vec))
    } else if let Ok(dict) = obj.downcast::<PyDict>() {
        Ok(Value::Object(py_dict_to_value_map(dict)?))
    } else {
        Err(PyValueError::new_err("Unsupported type"))
    }
}

fn value_to_py(py: Python, value: &Value) -> PyResult<PyObject> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.into_py(py)),
        Value::Integer(i) => Ok(i.into_py(py)),
        Value::Float(f) => Ok(f.into_py(py)),
        Value::String(s) => Ok(s.into_py(py)),
        Value::Array(vec) => {
            let list = PyList::empty(py);
            for item in vec {
                list.append(value_to_py(py, item)?)?;
            }
            Ok(list.into())
        }
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, value_to_py(py, v)?)?;
            }
            Ok(dict.into())
        }
    }
}

fn value_map_to_py_dict<'py>(py: Python<'py>, map: &HashMap<String, Value>) -> PyResult<&'py PyDict> {
    let dict = PyDict::new(py);
    for (k, v) in map {
        dict.set_item(k, value_to_py(py, v)?)?;
    }
    Ok(dict)
}

#[pymodule]
fn sdag(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyDag>()?;
    m.add_function(wrap_pyfunction!(load_dag_from_yaml, m)?)?;
    Ok(())
}