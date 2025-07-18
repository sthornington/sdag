use once_cell::sync::Lazy;
use serde_yaml::Value;
use std::collections::HashMap;

/// Trait for evaluating a node on one row of input data.
pub trait Node {
    fn eval(&self, row: &HashMap<String, f64>) -> f64;
}

type BuilderFn = fn(&Value) -> Result<Box<dyn Node + Send + Sync>, String>;
static BUILDERS: Lazy<HashMap<String, BuilderFn>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("input".into(), build_input as BuilderFn);
    m.insert("const".into(), build_const as BuilderFn);
    m.insert("add".into(), build_add as BuilderFn);
    m.insert("mul".into(), build_mul as BuilderFn);
    m.insert("div".into(), build_div as BuilderFn);
    m
});

/// Build a Node graph from a YAML value.
pub fn build_node(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
    let map = v
        .as_mapping()
        .ok_or("Node spec must be a mapping".to_string())?;
    let kind = map
        .get(&Value::String("type".into()))
        .and_then(Value::as_str)
        .ok_or("Node spec missing 'type' field".to_string())?;
    let builder = BUILDERS
        .get(kind)
        .ok_or_else(|| format!("Unknown node type '{}'", kind))?;
    builder(v)
}

fn build_input(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
    let map = v.as_mapping().unwrap();
    let name = map
        .get(&Value::String("name".into()))
        .and_then(Value::as_str)
        .ok_or("Input node missing 'name'".to_string())?
        .to_string();
    Ok(Box::new(InputNodeImpl { name }))
}

fn build_const(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
    let map = v.as_mapping().unwrap();
    let value = map
        .get(&Value::String("value".into()))
        .and_then(Value::as_f64)
        .ok_or("Const node missing 'value'".to_string())?;
    Ok(Box::new(ConstNode { value }))
}

fn build_add(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
    let map = v.as_mapping().unwrap();
    let seq = map
        .get(&Value::String("children".into()))
        .and_then(Value::as_sequence)
        .ok_or("Add node missing 'children'".to_string())?;
    let mut children = Vec::with_capacity(seq.len());
    for c in seq {
        children.push(build_node(c)?);
    }
    Ok(Box::new(AddNode { children }))
}

fn build_mul(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
    let map = v.as_mapping().unwrap();
    let seq = map
        .get(&Value::String("children".into()))
        .and_then(Value::as_sequence)
        .ok_or("Mul node missing 'children'".to_string())?;
    let mut children = Vec::with_capacity(seq.len());
    for c in seq {
        children.push(build_node(c)?);
    }
    Ok(Box::new(MulNode { children }))
}

fn build_div(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
    let map = v.as_mapping().unwrap();
    let left = map
        .get(&Value::String("left".into()))
        .ok_or("Div node missing 'left'".to_string())?;
    let right = map
        .get(&Value::String("right".into()))
        .ok_or("Div node missing 'right'".to_string())?;
    Ok(Box::new(DivNode {
        left: build_node(left)?,
        right: build_node(right)?,
    }))
}

struct InputNodeImpl {
    name: String,
}
impl Node for InputNodeImpl {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        *row.get(&self.name).unwrap_or(&0.0)
    }
}

struct ConstNode {
    value: f64,
}
impl Node for ConstNode {
    fn eval(&self, _: &HashMap<String, f64>) -> f64 {
        self.value
    }
}

struct AddNode {
    children: Vec<Box<dyn Node + Send + Sync>>,
}
impl Node for AddNode {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|c| c.eval(row)).sum()
    }
}

struct MulNode {
    children: Vec<Box<dyn Node + Send + Sync>>,
}
impl Node for MulNode {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|c| c.eval(row)).product()
    }
}

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

/// Core sampler that runs the trigger vs outputs on rows.
pub struct SamplerCore {
    trigger: Box<dyn Node + Send + Sync>,
    outputs: Vec<Box<dyn Node + Send + Sync>>,
}
impl SamplerCore {
    pub fn new(trigger_yaml: &str, output_yamls: &[&str]) -> Result<Self, String> {
        let trigger_val: Value = serde_yaml::from_str(trigger_yaml).map_err(|e| e.to_string())?;
        let trigger = build_node(&trigger_val)?;
        let mut outputs = Vec::with_capacity(output_yamls.len());
        for &s in output_yamls {
            let v: Value = serde_yaml::from_str(s).map_err(|e| e.to_string())?;
            outputs.push(build_node(&v)?);
        }
        Ok(SamplerCore { trigger, outputs })
    }

    pub fn run(&self, rows: Vec<HashMap<String, f64>>) -> Vec<HashMap<String, f64>> {
        let mut results = Vec::new();
        let mut prev: Option<f64> = None;
        for row in rows {
            let tval = self.trigger.eval(&row);
            if prev.map_or(true, |p| p != tval) {
                let mut rec = HashMap::new();
                rec.insert("trigger".to_string(), tval);
                for (i, node) in self.outputs.iter().enumerate() {
                    rec.insert(format!("output{}", i), node.eval(&row));
                }
                results.push(rec);
                prev = Some(tval);
            }
        }
        results
    }
}
