use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::DagError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

pub trait Node: Send + Sync {
    fn compute(&self, inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>>;
    
    fn input_schema(&self) -> Vec<(String, String)> {
        vec![]
    }
    
    fn output_schema(&self) -> Vec<(String, String)> {
        vec![]
    }
}

pub type NodeFactory = Arc<dyn Fn(HashMap<String, Value>) -> Result<Box<dyn Node>> + Send + Sync>;

pub struct NodeRegistry {
    factories: HashMap<String, NodeFactory>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
        };
        registry.register_default_nodes();
        registry
    }

    pub fn register(&mut self, name: &str, factory: NodeFactory) {
        self.factories.insert(name.to_string(), factory);
    }

    pub fn create(&self, node_type: &str, params: HashMap<String, Value>) -> Result<Box<dyn Node>> {
        self.factories
            .get(node_type)
            .ok_or_else(|| DagError::NodeNotFound(node_type.to_string()).into())
            .and_then(|factory| factory(params))
    }

    fn register_default_nodes(&mut self) {
        // Add node
        self.register("Add", Arc::new(|_params| {
            Ok(Box::new(AddNode) as Box<dyn Node>)
        }));

        // Multiply node
        self.register("Multiply", Arc::new(|_params| {
            Ok(Box::new(MultiplyNode) as Box<dyn Node>)
        }));

        // Constant node
        self.register("Constant", Arc::new(|params| {
            let value = params.get("value")
                .ok_or_else(|| DagError::InvalidInput("Constant node requires 'value' parameter".to_string()))?
                .clone();
            Ok(Box::new(ConstantNode { value }) as Box<dyn Node>)
        }));
    }
}

// Example nodes
struct AddNode;

impl Node for AddNode {
    fn compute(&self, inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>> {
        let a = inputs.get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| DagError::InvalidInput("Input 'a' must be a number".to_string()))?;
        
        let b = inputs.get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| DagError::InvalidInput("Input 'b' must be a number".to_string()))?;
        
        let mut outputs = HashMap::new();
        outputs.insert("result".to_string(), Value::Float(a + b));
        Ok(outputs)
    }

    fn input_schema(&self) -> Vec<(String, String)> {
        vec![
            ("a".to_string(), "number".to_string()),
            ("b".to_string(), "number".to_string()),
        ]
    }

    fn output_schema(&self) -> Vec<(String, String)> {
        vec![("result".to_string(), "number".to_string())]
    }
}

struct MultiplyNode;

impl Node for MultiplyNode {
    fn compute(&self, inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>> {
        let a = inputs.get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| DagError::InvalidInput("Input 'a' must be a number".to_string()))?;
        
        let b = inputs.get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| DagError::InvalidInput("Input 'b' must be a number".to_string()))?;
        
        let mut outputs = HashMap::new();
        outputs.insert("result".to_string(), Value::Float(a * b));
        Ok(outputs)
    }

    fn input_schema(&self) -> Vec<(String, String)> {
        vec![
            ("a".to_string(), "number".to_string()),
            ("b".to_string(), "number".to_string()),
        ]
    }

    fn output_schema(&self) -> Vec<(String, String)> {
        vec![("result".to_string(), "number".to_string())]
    }
}

struct ConstantNode {
    value: Value,
}

impl Node for ConstantNode {
    fn compute(&self, _inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>> {
        let mut outputs = HashMap::new();
        outputs.insert("value".to_string(), self.value.clone());
        Ok(outputs)
    }

    fn output_schema(&self) -> Vec<(String, String)> {
        vec![("value".to_string(), "any".to_string())]
    }
}