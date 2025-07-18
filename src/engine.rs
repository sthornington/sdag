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
    /// Parse an instance from its YAML mapping.
    fn from_yaml(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String>;
}

/// Build functions index: maps TYPE → NodeDef::from_yaml.
pub type BuilderFn = fn(&Value) -> Result<Box<dyn Node + Send + Sync>, String>;

/// Registration record for a YAML→Node builder
/// Registration record for a YAML→Node builder
pub struct Builder {
    pub tag: &'static str,
    pub build: BuilderFn,
}
// Collect all Builder registrations into an inventory registry
inventory::collect!(Builder);

/// Dispatch to the builder based on the YAML `type` field.
pub fn build_node(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
    let map = v
        .as_mapping()
        .ok_or_else(|| "Node spec must be a mapping".to_string())?;
    let kind = map
        .get(&Value::String("type".into()))
        .and_then(Value::as_str)
        .ok_or_else(|| "Node spec missing 'type' field".to_string())?;
    // Look up a matching builder from the inventory
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
impl Node for InputNodeImpl {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        *row.get(&self.name).unwrap_or(&0.0)
    }
}
impl NodeDef for InputNodeImpl {
    const TYPE: &'static str = "input";
    fn from_yaml(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
        let map = v
            .as_mapping()
            .ok_or_else(|| "Input node spec not mapping".to_string())?;
        let name = map
            .get(&Value::String("name".into()))
            .and_then(Value::as_str)
            .ok_or_else(|| "Input node missing 'name'".to_string())?
            .to_string();
        Ok(Box::new(InputNodeImpl { name }))
    }
}

/// Const node: always returns a constant.
pub struct ConstNode {
    pub value: f64,
}
impl Node for ConstNode {
    fn eval(&self, _: &HashMap<String, f64>) -> f64 {
        self.value
    }
}
impl NodeDef for ConstNode {
    const TYPE: &'static str = "const";
    fn from_yaml(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
        let map = v
            .as_mapping()
            .ok_or_else(|| "Const node spec not mapping".to_string())?;
        let value = map
            .get(&Value::String("value".into()))
            .and_then(Value::as_f64)
            .ok_or_else(|| "Const node missing 'value'".to_string())?;
        Ok(Box::new(ConstNode { value }))
    }
}

/// Add node: sums children.
pub struct AddNode {
    pub children: Vec<Box<dyn Node + Send + Sync>>,
}
impl Node for AddNode {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|c| c.eval(row)).sum()
    }
}
impl NodeDef for AddNode {
    const TYPE: &'static str = "add";
    fn from_yaml(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
        let map = v
            .as_mapping()
            .ok_or_else(|| "Add node spec not mapping".to_string())?;
        let seq = map
            .get(&Value::String("children".into()))
            .and_then(Value::as_sequence)
            .ok_or_else(|| "Add node missing 'children'".to_string())?;
        let mut children = Vec::with_capacity(seq.len());
        for c in seq {
            children.push(build_node(c)?);
        }
        Ok(Box::new(AddNode { children }))
    }
}

/// Mul node: multiplies children.
pub struct MulNode {
    pub children: Vec<Box<dyn Node + Send + Sync>>,
}
impl Node for MulNode {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        self.children.iter().map(|c| c.eval(row)).product()
    }
}
impl NodeDef for MulNode {
    const TYPE: &'static str = "mul";
    fn from_yaml(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
        let map = v
            .as_mapping()
            .ok_or_else(|| "Mul node spec not mapping".to_string())?;
        let seq = map
            .get(&Value::String("children".into()))
            .and_then(Value::as_sequence)
            .ok_or_else(|| "Mul node missing 'children'".to_string())?;
        let mut children = Vec::with_capacity(seq.len());
        for c in seq {
            children.push(build_node(c)?);
        }
        Ok(Box::new(MulNode { children }))
    }
}

/// Div node: left / right.
pub struct DivNode {
    pub left: Box<dyn Node + Send + Sync>,
    pub right: Box<dyn Node + Send + Sync>,
}
impl Node for DivNode {
    fn eval(&self, row: &HashMap<String, f64>) -> f64 {
        let l = self.left.eval(row);
        let r = self.right.eval(row);
        l / r
    }
}
impl NodeDef for DivNode {
    const TYPE: &'static str = "div";
    fn from_yaml(v: &Value) -> Result<Box<dyn Node + Send + Sync>, String> {
        let map = v
            .as_mapping()
            .ok_or_else(|| "Div node spec not mapping".to_string())?;
        let left = map
            .get(&Value::String("left".into()))
            .ok_or_else(|| "Div node missing 'left'".to_string())?;
        let right = map
            .get(&Value::String("right".into()))
            .ok_or_else(|| "Div node missing 'right'".to_string())?;
        Ok(Box::new(DivNode {
            left: build_node(left)?,
            right: build_node(right)?,
        }))
    }
}

/// Core sampler that runs trigger vs outputs on rows.
pub struct SamplerCore {
    trigger: Box<dyn Node + Send + Sync>,
    outputs: Vec<Box<dyn Node + Send + Sync>>,
}

// Register all built-in node builders into the inventory
inventory::submit! {
    Builder { tag: InputNodeImpl::TYPE, build: InputNodeImpl::from_yaml }
}
inventory::submit! {
    Builder { tag: ConstNode::TYPE, build: ConstNode::from_yaml }
}
inventory::submit! {
    Builder { tag: AddNode::TYPE, build: AddNode::from_yaml }
}
inventory::submit! {
    Builder { tag: MulNode::TYPE, build: MulNode::from_yaml }
}
inventory::submit! {
    Builder { tag: DivNode::TYPE, build: DivNode::from_yaml }
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
