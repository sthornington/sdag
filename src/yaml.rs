use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// YAML representation of a DAG
#[derive(Debug, Serialize, Deserialize)]
pub struct DagYaml {
    pub nodes: Vec<NodeYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeYaml {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default)]
    pub params: HashMap<String, Value>,
}