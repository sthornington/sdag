use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Node ID in the arena (index-based)
pub type NodeId = usize;

/// Arena-based graph storage
#[derive(Debug, Clone)]
pub struct Arena<T> {
    nodes: Vec<T>,
    shared_refs: HashMap<String, NodeId>, // Track shared nodes by original ID
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            shared_refs: HashMap::new(),
        }
    }
    
    pub fn insert(&mut self, node: T, original_id: Option<String>) -> NodeId {
        if let Some(id) = original_id {
            if let Some(&existing_id) = self.shared_refs.get(&id) {
                return existing_id;
            }
            let node_id = self.nodes.len();
            self.nodes.push(node);
            self.shared_refs.insert(id, node_id);
            node_id
        } else {
            let node_id = self.nodes.len();
            self.nodes.push(node);
            node_id
        }
    }
    
    pub fn get(&self, id: NodeId) -> Option<&T> {
        self.nodes.get(id)
    }
    
    pub fn nodes(&self) -> &[T] {
        &self.nodes
    }
    
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

/// Arena node representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaNode {
    pub id: NodeId,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(flatten)]
    pub data: serde_yaml::Value,
}

/// Complete graph with arena storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaGraph {
    pub nodes: Vec<ArenaNode>,
    pub root: NodeId,
}

impl ArenaGraph {
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml).map_err(|e| e.to_string())
    }
    
    pub fn to_yaml(&self) -> Result<String, String> {
        serde_yaml::to_string(self).map_err(|e| e.to_string())
    }
}