#[macro_use]
extern crate inventory;
mod engine;
use engine::{AddNode, ConstNode, DivNode, InputNodeImpl, MulNode, NodeDef, SamplerCore, extract_node_spec};
// procedural macro to generate Python node wrappers
use py_node_macro::py_node;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PySequence;
// pyo3::wrap_pyfunction no longer used
use serde_yaml::{Mapping, Value};
use std::collections::HashMap;

/// Python bindings and top-level module definitions.
#[pymodule]
fn sdag(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<InputNode>()?;
    m.add_class::<Const>()?;
    m.add_class::<Add>()?;
    m.add_class::<Mul>()?;
    m.add_class::<Div>()?;
    m.add_class::<Graph>()?;
    m.add_class::<Sampler>()?;
    // m.add_function(wrap_pyfunction!(freeze, m)?)?; // freeze() moved to Phase 2
    Ok(())
}

/// Python InputNode wrapper (ID node with scalar name).
#[py_node(InputNodeImpl::TYPE, name)]
#[pyclass(name = "InputNode", text_signature = "(id, name)")]
struct InputNode {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    name: String,
}

/// Python Const wrapper (ID node with scalar value).
#[py_node(ConstNode::TYPE, value)]
#[pyclass(name = "Const", text_signature = "(id, value)")]
struct Const {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    value: f64,
}

/// Python Add wrapper (ID node with upstream input IDs).
#[py_node(AddNode::TYPE, children)]
#[pyclass(name = "Add", text_signature = "(id, children)")]
struct Add {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    children: Vec<PyObject>,
}

/// Python Mul wrapper (ID node with upstream input IDs).
#[py_node(MulNode::TYPE, children)]
#[pyclass(name = "Mul", text_signature = "(id, children)")]
struct Mul {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    children: Vec<PyObject>,
}

/// Python Div wrapper (ID node with upstream input IDs).
#[py_node(DivNode::TYPE, left, right)]
#[pyclass(name = "Div", text_signature = "(id, left, right)")]
struct Div {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    left: PyObject,
    #[pyo3(get)]
    right: PyObject,
}

/// Python Graph (factory) wrapper storing nodes by unique ID.
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
    /// Create an InputNode, register it, and return the Python node object.
    #[pyo3(signature = (name))]
    fn input(&mut self, py: Python, name: String) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = InputNode {
            id: id.clone(),
            name,
        };
        let py_node = node.into_py(py);
        self.registry.insert(id.clone(), py_node.clone().into());
        py_node.into()
    }
    /// Create a Const, register it, and return the Python node object.
    #[pyo3(signature = (value))]
    fn r#const(&mut self, py: Python, value: f64) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Const {
            id: id.clone(),
            value,
        };
        let py_node = node.into_py(py);
        self.registry.insert(id.clone(), py_node.clone().into());
        py_node.into()
    }
    /// Create an Add node with upstream inputs, register it, and return the Python node object.
    #[pyo3(signature = (children))]
    fn add(&mut self, py: Python, children: Vec<PyObject>) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Add { id: id.clone(), children };
        let py_node = node.into_py(py);
        self.registry.insert(id.clone(), py_node.clone().into());
        py_node.into()
    }
    /// Create a Mul node with upstream inputs, register it, and return the Python node object.
    #[pyo3(signature = (children))]
    fn mul(&mut self, py: Python, children: Vec<PyObject>) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Mul { id: id.clone(), children };
        let py_node = node.into_py(py);
        self.registry.insert(id.clone(), py_node.clone().into());
        py_node.into()
    }
    /// Create a Div node with upstream inputs, register it, and return the Python node object.
    #[pyo3(signature = (left, right))]
    fn div(&mut self, py: Python, left: PyObject, right: PyObject) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Div { id: id.clone(), left, right };
        let py_node = node.into_py(py);
        self.registry.insert(id.clone(), py_node.clone().into());
        py_node.into()
    }

    /// Freeze the graph (reachable from `root`) into a flat YAML spec.
    #[pyo3(signature = (root))]
    fn freeze(&self, py: Python, root: PyObject) -> PyResult<String> {
        // collect nodes reachable from root via declared fields on the Python node objects
        let mut seen = Vec::new();
        let root_id: String = root.as_ref(py).getattr("id")?.extract()?;
        let mut stack = vec![root.clone()];
        while let Some(obj) = stack.pop() {
            let id: String = obj.as_ref(py).getattr("id")?.extract()?;
            if seen.contains(&id) {
                continue;
            }
            seen.push(id.clone());
            // collect dependencies via declared FIELDS
            let cls = obj.as_ref(py).get_type();
            if let Ok(fields) = cls.getattr("FIELDS") {
                if let Ok(field_names) = fields.extract::<Vec<String>>() {
                    for field in field_names {
                        let val = obj.as_ref(py).getattr(field.as_str())?;
                        // sequence of dependencies
                        if let Ok(seq) = val.cast_as::<PySequence>() {
                            for item in seq.iter()? {
                                let child_any = item?;
                                if let Ok(child) = child_any.extract::<PyObject>() {
                                    if child.as_ref(py).get_type().getattr("TYPE").is_ok() {
                                        stack.push(child);
                                    }
                                }
                            }
                        } else {
                            // single dependency
                            if let Ok(child) = val.extract::<PyObject>() {
                                if child.as_ref(py).get_type().getattr("TYPE").is_ok() {
                                    stack.push(child);
                                }
                            }
                        }
                    }
                }
            }
        }
        // produce topological order (reverse of DFS visitation)
        seen.reverse();
        let mut nodes_map = Mapping::new();
        for id in seen {
            let obj = self
                .registry
                .get(&id)
                .ok_or_else(|| PyValueError::new_err(format!("Unknown node ID '{}'", id)))?;
            let mut m = Mapping::new();
            let cls = obj.as_ref(py).get_type();
            let tag: String = cls.getattr("TYPE")?.extract()?;
            m.insert(Value::String("type".into()), Value::String(tag));
            // serialize fields per declared FIELDS
            let fields: Vec<String> = cls.getattr("FIELDS")?.extract()?;
            for field in fields {
                let val = obj.as_ref(py).getattr(field.as_str())?;
                let entry = if let Ok(children) = val.extract::<Vec<PyObject>>() {
                    let mut seq = Vec::new();
                    for child in children {
                        let cid: String = child.as_ref(py).getattr("id")?.extract()?;
                        seq.push(Value::String(cid));
                    }
                    Value::Sequence(seq)
                } else if let Ok(s) = val.extract::<String>() {
                    Value::String(s)
                } else if let Ok(f) = val.extract::<f64>() {
                    serde_yaml::to_value(f).map_err(|e| PyValueError::new_err(e.to_string()))?
                } else if let Ok(child) = val.extract::<PyObject>() {
                    // single upstream node
                    let cid: String = child.as_ref(py).getattr("id")?.extract()?;
                    Value::String(cid)
                } else {
                    return Err(PyValueError::new_err(format!(
                        "Unsupported field '{}' on node '{}'",
                        field, id
                    )));
                };
                m.insert(Value::String(field), entry);
            }
            nodes_map.insert(Value::String(id), Value::Mapping(m));
        }
        let mut top = Mapping::new();
        top.insert(Value::String("nodes".into()), Value::Mapping(nodes_map));
        top.insert(
            Value::String("root".into()),
            Value::String(root.as_ref(py).getattr("id")?.extract()?),
        );
        // serialize to YAML and trim trailing newline for embedding
        let yaml = serde_yaml::to_string(&Value::Mapping(top))
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(yaml.trim_end_matches('\n').to_string())
    }
}

/// Python Sampler wrapper (delegates to core engine).
#[pyclass]
struct Sampler {
    core: SamplerCore,
}
#[pymethods]
impl Sampler {
    #[new]
    #[pyo3(signature = (trigger, output))]
    fn new(trigger: &str, output: Vec<&str>) -> PyResult<Self> {
        // Normalize YAML specs: accept full graph spec or single-node spec and convert to single-node YAML
        fn normalize_spec(yml: &str) -> PyResult<String> {
            let val: Value = serde_yaml::from_str(yml).map_err(|e| PyValueError::new_err(e.to_string()))?;
            let spec = extract_node_spec(&val).map_err(|e| PyValueError::new_err(e))?;
            let s = serde_yaml::to_string(&spec).map_err(|e| PyValueError::new_err(e.to_string()))?;
            Ok(s.trim_end_matches('\n').to_string())
        }
        let trigger_spec = normalize_spec(trigger)?;
        let mut out_specs = Vec::with_capacity(output.len());
        for &o in &output {
            out_specs.push(normalize_spec(o)?);
        }
        let out_refs: Vec<&str> = out_specs.iter().map(|s| s.as_str()).collect();
        let core = SamplerCore::new(&trigger_spec, &out_refs).map_err(|e| PyValueError::new_err(e))?;
        Ok(Sampler { core })
    }

    fn run(&self, rows: Vec<HashMap<String, f64>>) -> PyResult<Vec<HashMap<String, f64>>> {
        Ok(self.core.run(rows))
    }
}

// NOTE: freeze() will be reimplemented in Phase 2 for arena/ID flattening.
