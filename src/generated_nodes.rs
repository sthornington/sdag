// Auto-generated node definitions from nodes.yaml
// DO NOT EDIT - run generate_nodes.py to regenerate

use crate::simple_node_macro::{EvalNode, ArenaEval};
use crate::engine::{NodeId, ArenaNode, FieldValue};
use std::collections::HashMap;
use pyo3::prelude::*;

// Input node
#[derive(Debug, Clone)]
pub struct InputNode {
    pub name: String,
}

impl EvalNode for InputNode {
    fn eval(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        *inputs.get(&self.name).unwrap_or(&0.0)
    }
}

impl ArenaEval for InputNode {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.eval(values, inputs)
    }
}

#[pyclass]
pub struct Input {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub name: String,
}

// Const node
#[derive(Debug, Clone)]
pub struct ConstNode {
    pub value: f64,
}

impl EvalNode for ConstNode {
    fn eval(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.value
    }
}

impl ArenaEval for ConstNode {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.eval(values, inputs)
    }
}

#[pyclass]
pub struct Const {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub value: f64,
}

// Add node
#[derive(Debug, Clone)]
pub struct AddNode {
    pub children: Vec<NodeId>,
}

impl EvalNode for AddNode {
    fn eval(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|&id| values[id]).sum()
    }
}

impl ArenaEval for AddNode {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.eval(values, inputs)
    }
}

#[pyclass]
pub struct Add {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub children: Vec<PyObject>,
}

// Mul node
#[derive(Debug, Clone)]
pub struct MulNode {
    pub children: Vec<NodeId>,
}

impl EvalNode for MulNode {
    fn eval(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|&id| values[id]).product()
    }
}

impl ArenaEval for MulNode {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.eval(values, inputs)
    }
}

#[pyclass]
pub struct Mul {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub children: Vec<PyObject>,
}

// Div node
#[derive(Debug, Clone)]
pub struct DivNode {
    pub left: NodeId,
    pub right: NodeId,
}

impl EvalNode for DivNode {
    fn eval(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        let l = values[self.left];
let r = values[self.right];
if r == 0.0 { f64::NAN } else { l / r }
    }
}

impl ArenaEval for DivNode {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.eval(values, inputs)
    }
}

#[pyclass]
pub struct Div {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub left: PyObject,
    #[pyo3(get)]
    pub right: PyObject,
}


// Graph builder methods
#[pymethods]
impl crate::Graph {
    pub fn input(&mut self, py: Python, name: String) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Input { id: id.clone(), name };
        let py_node = node.into_py(py);
        self.registry.insert(id, py_node.clone());
        py_node
    }

    #[pyo3(name = "const")]
    pub fn const_(&mut self, py: Python, value: f64) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Const { id: id.clone(), value };
        let py_node = node.into_py(py);
        self.registry.insert(id, py_node.clone());
        py_node
    }

    pub fn add(&mut self, py: Python, children: Vec<PyObject>) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Add { id: id.clone(), children };
        let py_node = node.into_py(py);
        self.registry.insert(id, py_node.clone());
        py_node
    }

    pub fn mul(&mut self, py: Python, children: Vec<PyObject>) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Mul { id: id.clone(), children };
        let py_node = node.into_py(py);
        self.registry.insert(id, py_node.clone());
        py_node
    }

    pub fn div(&mut self, py: Python, left: PyObject, right: PyObject) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = Div { id: id.clone(), left, right };
        let py_node = node.into_py(py);
        self.registry.insert(id, py_node.clone());
        py_node
    }

}

// Arena builder
pub fn build_arena_node(node: &ArenaNode) -> Result<Box<dyn ArenaEval>, String> {
    match node.tag.as_str() {
        "input" => {
            let name = match node.fields.get("name") {
                Some(FieldValue::Str(s)) => s.clone(),
                _ => return Err("node missing name".to_string()),
            };
            Ok(Box::new(InputNode { name }))
        },
        "const" => {
            let value = match node.fields.get("value") {
                Some(FieldValue::Float(f)) => *f,
                _ => return Err("node missing value".to_string()),
            };
            Ok(Box::new(ConstNode { value }))
        },
        "add" => {
            let children = match node.fields.get("children") {
                Some(FieldValue::Many(ids)) => ids.clone(),
                _ => return Err("node missing children".to_string()),
            };
            Ok(Box::new(AddNode { children }))
        },
        "mul" => {
            let children = match node.fields.get("children") {
                Some(FieldValue::Many(ids)) => ids.clone(),
                _ => return Err("node missing children".to_string()),
            };
            Ok(Box::new(MulNode { children }))
        },
        "div" => {
            let left = match node.fields.get("left") {
                Some(FieldValue::One(id)) => *id,
                _ => return Err("node missing left".to_string()),
            };
            let right = match node.fields.get("right") {
                Some(FieldValue::One(id)) => *id,
                _ => return Err("node missing right".to_string()),
            };
            Ok(Box::new(DivNode { left, right }))
        },
        _ => Err(format!("Unknown node type: {}", node.tag)),
    }
}

// Python registration
pub fn register_nodes(m: &pyo3::types::PyModule) -> PyResult<()> {
    m.add_class::<Input>()?;
    m.add_class::<Const>()?;
    m.add_class::<Add>()?;
    m.add_class::<Mul>()?;
    m.add_class::<Div>()?;
    Ok(())
}

// Freeze helper
pub fn freeze_node_fields(py: Python, obj: &PyObject, node_type: &str, mapping: &mut serde_yaml::Mapping, id2idx: &HashMap<String, usize>) -> PyResult<()> {
    use serde_yaml::Value;
    
    match node_type {
        "input" => {
            let name: String = obj.as_ref(py).getattr("name")?.extract()?;
            mapping.insert(Value::String("name".into()), Value::String(name));
        },
        "const" => {
            let value: f64 = obj.as_ref(py).getattr("value")?.extract()?;
            mapping.insert(Value::String("value".into()), serde_yaml::to_value(value).unwrap());
        },
        "add" => {
            let children: Vec<PyObject> = obj.as_ref(py).getattr("children")?.extract()?;
            let mut idxs = Vec::new();
            for child in children {
                let cid: String = child.as_ref(py).getattr("id")?.extract()?;
                idxs.push(Value::Number(serde_yaml::Number::from(id2idx[&cid] as i64)));
            }
            mapping.insert(Value::String("children".into()), Value::Sequence(idxs));
        },
        "mul" => {
            let children: Vec<PyObject> = obj.as_ref(py).getattr("children")?.extract()?;
            let mut idxs = Vec::new();
            for child in children {
                let cid: String = child.as_ref(py).getattr("id")?.extract()?;
                idxs.push(Value::Number(serde_yaml::Number::from(id2idx[&cid] as i64)));
            }
            mapping.insert(Value::String("children".into()), Value::Sequence(idxs));
        },
        "div" => {
            let left: PyObject = obj.as_ref(py).getattr("left")?.extract()?;
            let left_id: String = left.as_ref(py).getattr("id")?.extract()?;
            mapping.insert(Value::String("left".into()), Value::Number(serde_yaml::Number::from(id2idx[&left_id] as i64)));
            let right: PyObject = obj.as_ref(py).getattr("right")?.extract()?;
            let right_id: String = right.as_ref(py).getattr("id")?.extract()?;
            mapping.insert(Value::String("right".into()), Value::Number(serde_yaml::Number::from(id2idx[&right_id] as i64)));
        },
        _ => {},
    }
    Ok(())
}
