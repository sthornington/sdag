use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::{DagBuilder, Dag, NodeRegistry, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct DagYaml {
    pub nodes: Vec<NodeYaml>,
    pub connections: Vec<ConnectionYaml>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeYaml {
    pub id: String,
    pub node_type: String,
    #[serde(default)]
    pub params: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionYaml {
    pub from_node: String,
    pub from_output: String,
    pub to_node: String,
    pub to_input: String,
}

impl DagYaml {
    pub fn to_dag(&self, registry: Arc<NodeRegistry>) -> Result<Dag> {
        let mut builder = DagBuilder::new(registry);

        // Add nodes
        for node in &self.nodes {
            builder.add_node(node.id.clone(), &node.node_type, node.params.clone())?;
        }

        // Add connections
        for conn in &self.connections {
            builder.connect(
                &conn.from_node, &conn.from_output,
                &conn.to_node, &conn.to_input
            )?;
        }

        builder.build()
    }

    pub fn from_yaml(yaml_str: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(yaml_str)?)
    }

    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }
}

pub fn load_dag_from_yaml(yaml_str: &str, registry: Arc<NodeRegistry>) -> Result<Dag> {
    let dag_yaml = DagYaml::from_yaml(yaml_str)?;
    dag_yaml.to_dag(registry)
}

pub fn save_dag_to_yaml(dag_yaml: &DagYaml) -> Result<String> {
    dag_yaml.to_yaml()
}