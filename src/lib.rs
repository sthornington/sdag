mod engine;
use engine::{AddNode, ConstNode, DivNode, InputNodeImpl, MulNode, NodeDef, SamplerCore};
//#[macro removed: py_node, PyNode derive]
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
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
#[pyclass(name = "InputNode")]
struct InputNode {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    name: String,
}
#[pymethods]
impl InputNode {
    #[classattr]
    const TYPE: &'static str = InputNodeImpl::TYPE;
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
    #[classattr]
    const TYPE: &'static str = ConstNode::TYPE;
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
    #[classattr]
    const TYPE: &'static str = AddNode::TYPE;
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
    #[classattr]
    const TYPE: &'static str = MulNode::TYPE;
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
    #[classattr]
    const TYPE: &'static str = DivNode::TYPE;
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

    /// Freeze the graph (reachable from `root_id`) into a flat YAML spec.
    fn freeze(&self, py: Python, root_id: &str) -> PyResult<String> {
        // collect nodes reachable from root via inputs
        let mut seen = Vec::new();
        let mut stack = vec![root_id.to_string()];
        while let Some(id) = stack.pop() {
            if seen.contains(&id) {
                continue;
            }
            seen.push(id.clone());
            let obj = self
                .registry
                .get(&id)
                .ok_or_else(|| PyValueError::new_err(format!("Unknown node ID '{}'", id)))?;
            if let Ok(field) = obj.as_ref(py).getattr("inputs") {
                if let Ok(ids) = field.extract::<Vec<String>>() {
                    for cid in ids {
                        stack.push(cid);
                    }
                }
            }
        }
        // produce topological order (reverse of DFS visitation)
        seen.reverse();
        let mut nodes_map = Mapping::new();
        for id in seen {
            let obj = self.registry.get(&id).unwrap();
            let cls = obj.as_ref(py).get_type();
            let mut m = Mapping::new();
            let tag: String = cls.getattr("TYPE")?.extract()?;
            m.insert(Value::String("type".into()), Value::String(tag));
            if cls.is_subclass_of::<InputNode>()? {
                let name: String = obj.as_ref(py).getattr("name")?.extract()?;
                m.insert(Value::String("name".into()), Value::String(name));
            } else if cls.is_subclass_of::<Const>()? {
                let v: f64 = obj.as_ref(py).getattr("value")?.extract()?;
                m.insert(
                    Value::String("value".into()),
                    serde_yaml::to_value(v).map_err(|e| PyValueError::new_err(e.to_string()))?,
                );
            } else {
                let ids: Vec<String> = obj.as_ref(py).getattr("inputs")?.extract()?;
                let seq = ids.into_iter().map(Value::String).collect();
                m.insert(Value::String("inputs".into()), Value::Sequence(seq));
            }
            nodes_map.insert(Value::String(id), Value::Mapping(m));
        }
        let mut top = Mapping::new();
        top.insert(Value::String("nodes".into()), Value::Mapping(nodes_map));
        top.insert(Value::String("root".into()), Value::String(root_id.into()));
        serde_yaml::to_string(&Value::Mapping(top))
            .map_err(|e| PyValueError::new_err(e.to_string()))
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
