mod engine;
use engine::{SamplerCore, build_node};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use serde_yaml::Value;
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
struct InputNode {
    name: String,
}
#[pymethods]
impl InputNode {
    #[new]
    fn new(name: String) -> Self {
        InputNode { name }
    }
}

/// Python Const wrapper.
#[pyclass(name = "Const")]
struct Const {
    value: f64,
}
#[pymethods]
impl Const {
    #[new]
    fn new(value: f64) -> Self {
        Const { value }
    }
}

/// Python Add wrapper.
#[pyclass(name = "Add")]
struct Add {
    children: Vec<PyObject>,
}
#[pymethods]
impl Add {
    #[new]
    fn new(children: Vec<PyObject>) -> Self {
        Add { children }
    }
}

/// Python Mul wrapper.
#[pyclass(name = "Mul")]
struct Mul {
    children: Vec<PyObject>,
}
#[pymethods]
impl Mul {
    #[new]
    fn new(children: Vec<PyObject>) -> Self {
        Mul { children }
    }
}

/// Python Div wrapper.
#[pyclass(name = "Div")]
struct Div {
    left: PyObject,
    right: PyObject,
}
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
    // recurse through Python node wrappers and build serde_yaml::Value
    let py = obj.py();
    fn freeze_val(obj: &PyAny, py: Python) -> PyResult<Value> {
        use serde_yaml::Mapping;
        if let Ok(inp) = obj.extract::<PyRef<InputNode>>() {
            let mut m = Mapping::new();
            m.insert(Value::String("type".into()), Value::String("input".into()));
            m.insert(
                Value::String("name".into()),
                Value::String(inp.name.clone()),
            );
            return Ok(Value::Mapping(m));
        }
        if let Ok(c) = obj.extract::<PyRef<Const>>() {
            let mut m = Mapping::new();
            m.insert(Value::String("type".into()), Value::String("const".into()));
            let v_num =
                serde_yaml::to_value(c.value).map_err(|e| PyValueError::new_err(e.to_string()))?;
            m.insert(Value::String("value".into()), v_num);
            return Ok(Value::Mapping(m));
        }
        if let Ok(a) = obj.extract::<PyRef<Add>>() {
            let mut m = Mapping::new();
            m.insert(Value::String("type".into()), Value::String("add".into()));
            let mut seq = Vec::new();
            for child in &a.children {
                let val = freeze_val(child.as_ref(py), py)?;
                seq.push(val);
            }
            m.insert(Value::String("children".into()), Value::Sequence(seq));
            return Ok(Value::Mapping(m));
        }
        if let Ok(a) = obj.extract::<PyRef<Mul>>() {
            let mut m = Mapping::new();
            m.insert(Value::String("type".into()), Value::String("mul".into()));
            let mut seq = Vec::new();
            for child in &a.children {
                let val = freeze_val(child.as_ref(py), py)?;
                seq.push(val);
            }
            m.insert(Value::String("children".into()), Value::Sequence(seq));
            return Ok(Value::Mapping(m));
        }
        if let Ok(a) = obj.extract::<PyRef<Div>>() {
            let mut m = Mapping::new();
            m.insert(Value::String("type".into()), Value::String("div".into()));
            let l = freeze_val(a.left.as_ref(py), py)?;
            let r = freeze_val(a.right.as_ref(py), py)?;
            m.insert(Value::String("left".into()), l);
            m.insert(Value::String("right".into()), r);
            return Ok(Value::Mapping(m));
        }
        Err(PyValueError::new_err("Unsupported node type for freeze"))
    }
    let v = freeze_val(obj, py)?;
    serde_yaml::to_string(&v).map_err(|e| PyValueError::new_err(e.to_string()))
}
