use serde_yaml::Value;
use std::collections::HashMap;

/// Core evaluation trait: compute an f64 from one row of inputs.
pub trait Node {
    fn eval(&self, row: &HashMap<String, f64>) -> f64;
}

/// Extract a single-node spec (mapping with 'type') or unpack a full-graph spec by root
pub(crate) fn extract_node_spec(val: &Value) -> Result<Value, String> {
    let map = val.as_mapping().ok_or_else(|| "Spec must be a mapping".to_string())?;
    // single-node spec
    if map.contains_key(&Value::String("type".into())) {
        return Ok(val.clone());
    }
    // graph spec
    let root = map
        .get(&Value::String("root".into()))
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing 'root' in graph spec".to_string())?;
    let nodes = map
        .get(&Value::String("nodes".into()))
        .and_then(Value::as_mapping)
        .ok_or_else(|| "Missing 'nodes' in graph spec".to_string())?;
    nodes
        .get(&Value::String(root.into()))
        .cloned()
        .ok_or_else(|| format!("Graph spec: root '{}' not found", root))
}

/// Extension trait tying a YAML `type` tag to its builder.
pub trait NodeDef: Sized {
    /// The `type` tag used in YAML to identify this node.
    const TYPE: &'static str;
    /// Build from a YAML spec value (handled by the `#[py_node]` macro).
    fn from_yaml(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String>;
}

/// Dispatch to the builder based on the YAML `type` field.
pub type BuilderFn = fn(&Value) -> Result<Box<dyn Node + Send + Sync>, String>;
pub struct Builder {
    pub tag: &'static str,
    pub build: BuilderFn,
}
inventory::collect!(Builder);
pub fn build_node(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
    let map = v.as_mapping().ok_or_else(|| "Node spec must be a mapping".to_string())?;
    let kind = map
        .get(&Value::String("type".into()))
        .and_then(Value::as_str)
        .ok_or_else(|| "Node spec missing 'type' field".to_string())?;
    for b in inventory::iter::<Builder> {
        if b.tag == kind {
            return (b.build)(v);
        }
    }
    Err(format!("Unknown node type '{}'", kind))
}

/// Input node: reads a column by name.
pub struct InputNodeImpl {
    pub name: String,
}
impl InputNodeImpl {
    /// `type` tag for this node in YAML.
    pub const TYPE: &'static str = "input";
}
impl Node for InputNodeImpl {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        *row.get(&self.name).unwrap_or(&0.0)
    }
}

/// Const node: always returns a constant.
pub struct ConstNode {
    pub value: f64,
}
impl ConstNode {
    /// `type` tag for this node in YAML.
    pub const TYPE: &'static str = "const";
}
impl Node for ConstNode {
    fn eval(&self, _: &HashMap<String, f64>) -> f64 {
        self.value
    }
}

/// Add node: sums children.
pub struct AddNode {
    pub children: Vec<Box<dyn Node + Send + Sync>>,
}
impl AddNode {
    /// `type` tag for this node in YAML.
    pub const TYPE: &'static str = "add";
}
impl Node for AddNode {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|c| c.eval(row)).sum()
    }
}

/// Mul node: multiplies children.
pub struct MulNode {
    pub children: Vec<Box<dyn Node + Send + Sync>>,
}
impl MulNode {
    /// `type` tag for this node in YAML.
    pub const TYPE: &'static str = "mul";
}
impl Node for MulNode {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|c| c.eval(row)).product()
    }
}

/// Div node: left / right.
pub struct DivNode {
    pub left: Box<dyn Node + Send + Sync>,
    pub right: Box<dyn Node + Send + Sync>,
}
impl DivNode {
    /// `type` tag for this node in YAML.
    pub const TYPE: &'static str = "div";
}
impl Node for DivNode {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        let l = self.left.eval(row);
        let r = self.right.eval(row);
        l / r
    }
}
pub struct SamplerCore {
    trigger: Box<dyn Node + Send + Sync>,
    outputs: Vec<Box<dyn Node + Send + Sync>>,
}

impl SamplerCore {
    pub fn new(trigger_yaml: &str, output_yamls: &[&str]) -> Result<Self, String> {
        // unwrap either a single-node or full graph spec for trigger
        let tval: Value = serde_yaml::from_str(trigger_yaml).map_err(|e| e.to_string())?;
        let trigger_spec = extract_node_spec(&tval)
            .map_err(|e| format!("Invalid trigger spec: {}", e))?;
        let trigger = build_node(&trigger_spec)?;

        // unwrap each output spec similarly
        let mut outputs = Vec::with_capacity(output_yamls.len());
        for &yml in output_yamls {
            let oval: Value = serde_yaml::from_str(yml).map_err(|e| e.to_string())?;
            let spec = extract_node_spec(&oval)
                .map_err(|e| format!("Invalid output spec: {}", e))?;
            outputs.push(build_node(&spec)?);
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
