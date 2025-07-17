use once_cell::sync::Lazy;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use serde_yaml::Value;
use std::collections::HashMap;

/// Trait for evaluating a node against a row of input values.
trait Node {
    /// Evaluate this node given a row (mapping from input names to values).
    fn eval(&self, row: &HashMap<String, f64>) -> f64;
}

/// Registry of node builders for parsing YAML descriptions.
type BuilderFn = fn(&Value) -> PyResult<Box<dyn Node + Send + Sync>>;
static BUILDERS: Lazy<HashMap<String, BuilderFn>> = Lazy::new(|| {
    let mut m: HashMap<String, BuilderFn> = HashMap::new();
    m.insert("input".into(), build_input as BuilderFn);
    m.insert("const".into(), build_const as BuilderFn);
    m.insert("add".into(), build_add as BuilderFn);
    m.insert("mul".into(), build_mul as BuilderFn);
    m.insert("div".into(), build_div as BuilderFn);
    m
});

/// Parse a YAML Value into a Node trait object recursively.
fn build_node(v: &Value) -> PyResult<Box<dyn Node + Send + Sync>> {
    let map = v.as_mapping().ok_or_else(|| PyValueError::new_err("Node spec must be a mapping"))?;
    let kind = map
        .get(&Value::String("type".into()))
        .and_then(Value::as_str)
        .ok_or_else(|| PyValueError::new_err("Node spec missing 'type' field"))?;
    let builder = BUILDERS
        .get(kind)
        .ok_or_else(|| PyValueError::new_err(format!("Unknown node type '{}'", kind)))?;
    builder(v)
}

/// Builder for Input node.
fn build_input(v: &Value) -> PyResult<Box<dyn Node + Send + Sync>> {
    let map = v.as_mapping().unwrap();
    let name = map
        .get(&Value::String("name".into()))
        .and_then(Value::as_str)
        .ok_or_else(|| PyValueError::new_err("Input node missing 'name'"))?
        .to_string();
    Ok(Box::new(InputNodeImpl { name }))
}

/// Builder for Const node.
fn build_const(v: &Value) -> PyResult<Box<dyn Node + Send + Sync>> {
    let map = v.as_mapping().unwrap();
    let value = map
        .get(&Value::String("value".into()))
        .and_then(Value::as_f64)
        .ok_or_else(|| PyValueError::new_err("Const node missing 'value'"))?;
    Ok(Box::new(ConstNode { value }))
}

/// Builder for Add node.
fn build_add(v: &Value) -> PyResult<Box<dyn Node + Send + Sync>> {
    let map = v.as_mapping().unwrap();
    let seq = map
        .get(&Value::String("children".into()))
        .and_then(Value::as_sequence)
        .ok_or_else(|| PyValueError::new_err("Add node missing 'children'"))?;
    let mut children: Vec<Box<dyn Node + Send + Sync>> = Vec::with_capacity(seq.len());
    for c in seq {
        children.push(build_node(c)?);
    }
    Ok(Box::new(AddNode { children }))
}

/// Builder for Mul node.
fn build_mul(v: &Value) -> PyResult<Box<dyn Node + Send + Sync>> {
    let map = v.as_mapping().unwrap();
    let seq = map
        .get(&Value::String("children".into()))
        .and_then(Value::as_sequence)
        .ok_or_else(|| PyValueError::new_err("Mul node missing 'children'"))?;
    let mut children: Vec<Box<dyn Node + Send + Sync>> = Vec::with_capacity(seq.len());
    for c in seq {
        children.push(build_node(c)?);
    }
    Ok(Box::new(MulNode { children }))
}

/// Builder for Div (division) node.
fn build_div(v: &Value) -> PyResult<Box<dyn Node + Send + Sync>> {
    let map = v.as_mapping().unwrap();
    let left = map
        .get(&Value::String("left".into()))
        .ok_or_else(|| PyValueError::new_err("Dic node missing 'left'"))?;
    let right = map
        .get(&Value::String("right".into()))
        .ok_or_else(|| PyValueError::new_err("Dic node missing 'right'"))?;
    Ok(Box::new(DivNode {
        left: build_node(left)?,
        right: build_node(right)?,
    }))
}

/// Input node (reads a column from input row).
struct InputNodeImpl {
    name: String,
}
impl Node for InputNodeImpl {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        *row.get(&self.name).unwrap_or(&0.0)
    }
}

/// Const node (constant value).
struct ConstNode {
    value: f64,
}
impl Node for ConstNode {
    fn eval(&self, _row: &HashMap<String, f64>) -> f64 {
        self.value
    }
}

/// Add node (sum of children).
struct AddNode {
    children: Vec<Box<dyn Node + Send + Sync>>,
}
impl Node for AddNode {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|c| c.eval(row)).sum()
    }
}

/// Mul node (product of children).
struct MulNode {
    children: Vec<Box<dyn Node + Send + Sync>>,
}
impl Node for MulNode {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|c| c.eval(row)).product()
    }
}

/// Dic node (division of left by right).
struct DivNode {
    left: Box<dyn Node + Send + Sync>,
    right: Box<dyn Node + Send + Sync>,
}
impl Node for DivNode {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        let l = self.left.eval(row);
        let r = self.right.eval(row);
        l / r
    }
}

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

/// Python Sampler wrapper.
#[pyclass]
struct Sampler {
    trigger: Box<dyn Node + Send + Sync>,
    outputs: Vec<Box<dyn Node + Send + Sync>>,
}
#[pymethods]
impl Sampler {
    #[new]
    #[pyo3(signature = (trigger, output))]
    fn new(trigger: &str, output: Vec<&str>) -> PyResult<Self> {
        let trigger_val: Value = serde_yaml::from_str(trigger)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let trigger = build_node(&trigger_val)?;
        let mut outputs = Vec::with_capacity(output.len());
        for s in output {
            let v: Value = serde_yaml::from_str(s)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            outputs.push(build_node(&v)?);
        }
        Ok(Sampler { trigger, outputs })
    }

    /// Run the sampler over rows. Returns rows at each trigger change.
    fn run(&self, rows: Vec<HashMap<String, f64>>) -> PyResult<Vec<HashMap<String, f64>>> {
        let mut out_rows = Vec::new();
        let mut prev = None;
        for row in rows {
            let val = self.trigger.eval(&row);
            if prev.map_or(true, |p| p != val) {
                // trigger changed
                let mut rec = HashMap::new();
                rec.insert("trigger".to_string(), val);
                for (i, node) in self.outputs.iter().enumerate() {
                    let ov = node.eval(&row);
                    rec.insert(format!("output{}", i), ov);
                }
                out_rows.push(rec);
                prev = Some(val);
            }
        }
        Ok(out_rows)
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
            m.insert(Value::String("name".into()), Value::String(inp.name.clone()));
            return Ok(Value::Mapping(m));
        }
        if let Ok(c) = obj.extract::<PyRef<Const>>() {
            let mut m = Mapping::new();
            m.insert(Value::String("type".into()), Value::String("const".into()));
            let v_num = serde_yaml::to_value(c.value)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
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
