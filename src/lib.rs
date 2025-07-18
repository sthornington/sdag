mod engine;
use engine::{AddNode, ConstNode, DivNode, InputNodeImpl, MulNode, NodeDef, SamplerCore};
use py_node_derive::{PyNode, py_node};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
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
    m.add_function(wrap_pyfunction!(freeze, m)?)?;
    Ok(())
}

/// Python InputNode wrapper.
#[pyclass(name = "InputNode")]
#[derive(PyNode)]
struct InputNode {
    #[pyo3(get)]
    name: String,
}
#[py_node(InputNodeImpl)]
#[pymethods]
impl InputNode {
    #[new]
    fn new(name: String) -> Self {
        InputNode { name }
    }
}

/// Python Const wrapper.
#[pyclass(name = "Const")]
#[derive(PyNode)]
struct Const {
    #[pyo3(get)]
    value: f64,
}
#[py_node(ConstNode)]
#[pymethods]
impl Const {
    #[new]
    fn new(value: f64) -> Self {
        Const { value }
    }
}

/// Python Add wrapper.
#[pyclass(name = "Add")]
#[derive(PyNode)]
struct Add {
    #[pyo3(get)]
    children: Vec<PyObject>,
}
#[py_node(AddNode)]
#[pymethods]
impl Add {
    #[new]
    fn new(children: Vec<PyObject>) -> Self {
        Add { children }
    }
}

/// Python Mul wrapper.
#[pyclass(name = "Mul")]
#[derive(PyNode)]
struct Mul {
    #[pyo3(get)]
    children: Vec<PyObject>,
}
#[py_node(MulNode)]
#[pymethods]
impl Mul {
    #[new]
    fn new(children: Vec<PyObject>) -> Self {
        Mul { children }
    }
}

/// Python Div wrapper.
#[pyclass(name = "Div")]
#[derive(PyNode)]
struct Div {
    #[pyo3(get)]
    left: PyObject,
    #[pyo3(get)]
    right: PyObject,
}
#[py_node(DivNode)]
#[pymethods]
impl Div {
    #[new]
    fn new(left: PyObject, right: PyObject) -> Self {
        Div { left, right }
    }
}

/// Python Graph (factory) wrapper.
#[pyclass]
struct Graph;
#[pymethods]
impl Graph {
    #[new]
    fn new() -> Self {
        Graph
    }
    fn add(&self, children: Vec<PyObject>) -> Add {
        Add { children }
    }
    fn r#const(&self, value: f64) -> Const {
        Const { value }
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
        let core = SamplerCore::new(trigger, &output).map_err(|e| PyValueError::new_err(e))?;
        Ok(Sampler { core })
    }

    fn run(&self, rows: Vec<HashMap<String, f64>>) -> PyResult<Vec<HashMap<String, f64>>> {
        Ok(self.core.run(rows))
    }
}

/// Freeze a node to a YAML description.
#[pyfunction]
fn freeze(obj: &PyAny) -> PyResult<String> {
    // Build a serde_yaml::Value spec for any PyNode wrapper based on its TYPE, FIELDS, and SEQ_FIELDS
    let py = obj.py();
    fn freeze_val(obj: &PyAny, py: Python) -> PyResult<Value> {
        let cls = obj.get_type();
        // tag name
        let tag: &str = cls.getattr("TYPE")?.extract()?;
        // field names and which are sequences
        let fields: Vec<String> = cls.getattr("FIELDS")?.extract()?;
        let seq_fields: Vec<String> = cls.getattr("SEQ_FIELDS")?.extract()?;
        let mut m = Mapping::new();
        m.insert(Value::String("type".into()), Value::String(tag.into()));
        for field in fields {
            let attr = obj.getattr(field.as_str())?;
            if seq_fields.contains(&field) {
                let list: Vec<PyObject> = attr.extract()?;
                let mut seq = Vec::with_capacity(list.len());
                for elt in list {
                    seq.push(freeze_val(elt.as_ref(py), py)?);
                }
                m.insert(Value::String(field.clone()), Value::Sequence(seq));
            } else if attr.get_type().getattr("TYPE").is_ok() {
                // nested node wrapper
                let nested = freeze_val(attr, py)?;
                m.insert(Value::String(field.clone()), nested);
            } else {
                // scalar value
                if let Ok(x) = attr.extract::<f64>() {
                    let v = serde_yaml::to_value(x)
                        .map_err(|e| PyValueError::new_err(e.to_string()))?;
                    m.insert(Value::String(field.clone()), v);
                } else if let Ok(s) = attr.extract::<String>() {
                    let v = serde_yaml::to_value(s)
                        .map_err(|e| PyValueError::new_err(e.to_string()))?;
                    m.insert(Value::String(field.clone()), v);
                } else {
                    return Err(PyValueError::new_err(format!(
                        "Unsupported field '{}' for freeze",
                        field
                    )));
                }
            }
        }
        Ok(Value::Mapping(m))
    }
    let v = freeze_val(obj, py)?;
    serde_yaml::to_string(&v).map_err(|e| PyValueError::new_err(e.to_string()))
}
