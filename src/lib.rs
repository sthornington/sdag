mod engine;
use engine::{AddNode, ConstNode, DivNode, InputNodeImpl, MulNode, NodeDef, SamplerCore};
//#[macro removed: py_node, PyNode derive]
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
// use pyo3::wrap_pyfunction;
// use serde_yaml::{Mapping, Value};
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
#[pyclass(name = "InputNode")]
struct InputNode {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    name: String,
}
#[pymethods]
impl InputNode {
    #[new]
    fn new(id: String, name: String) -> Self {
        InputNode { id, name }
    }
}

/// Python Const wrapper (ID node with scalar value).
#[pyclass(name = "Const")]
struct Const {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    value: f64,
}
#[pymethods]
impl Const {
    #[new]
    fn new(id: String, value: f64) -> Self {
        Const { id, value }
    }
}

/// Python Add wrapper (ID node with upstream input IDs).
#[pyclass(name = "Add")]
struct Add {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    inputs: Vec<String>,
}
#[pymethods]
impl Add {
    #[new]
    fn new(id: String, inputs: Vec<String>) -> Self {
        Add { id, inputs }
    }
}

/// Python Mul wrapper (ID node with upstream input IDs).
#[pyclass(name = "Mul")]
struct Mul {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    inputs: Vec<String>,
}
#[pymethods]
impl Mul {
    #[new]
    fn new(id: String, inputs: Vec<String>) -> Self {
        Mul { id, inputs }
    }
}

/// Python Div wrapper (ID node with upstream input IDs).
#[pyclass(name = "Div")]
struct Div {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    inputs: Vec<String>,
}
#[pymethods]
impl Div {
    #[new]
    fn new(id: String, inputs: Vec<String>) -> Self {
        Div { id, inputs }
    }
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
    /// Create an InputNode, register it, and return its ID.
    fn input(&mut self, py: Python, name: String) -> String {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = InputNode {
            id: id.clone(),
            name,
        };
        self.registry.insert(id.clone(), node.into_py(py));
        id
    }
    /// Create a Const, register it, and return its ID.
    fn r#const(&mut self, py: Python, value: f64) -> String {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Const {
            id: id.clone(),
            value,
        };
        self.registry.insert(id.clone(), node.into_py(py));
        id
    }
    /// Create an Add node with upstream IDs, register it, and return its ID.
    fn add(&mut self, py: Python, inputs: Vec<String>) -> String {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Add {
            id: id.clone(),
            inputs,
        };
        self.registry.insert(id.clone(), node.into_py(py));
        id
    }
    /// Create a Mul node with upstream IDs, register it, and return its ID.
    fn mul(&mut self, py: Python, inputs: Vec<String>) -> String {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Mul {
            id: id.clone(),
            inputs,
        };
        self.registry.insert(id.clone(), node.into_py(py));
        id
    }
    /// Create a Div node with upstream IDs, register it, and return its ID.
    fn div(&mut self, py: Python, inputs: Vec<String>) -> String {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Div {
            id: id.clone(),
            inputs,
        };
        self.registry.insert(id.clone(), node.into_py(py));
        id
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

// NOTE: freeze() will be reimplemented in Phase 2 for arena/ID flattening.
