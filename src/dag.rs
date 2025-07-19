use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use anyhow::Result;
use crate::{DagError, Node, NodeRegistry, Value};

pub struct DagNode {
    pub id: String,
    pub node: Box<dyn Node>,
    pub inputs: HashMap<String, Connection>,
}

#[derive(Clone)]
pub struct Connection {
    pub source_node: String,
    pub source_output: String,
}

pub struct Dag {
    nodes: HashMap<String, DagNode>,
    topological_order: Vec<String>,
}

impl Dag {
    fn new(nodes: HashMap<String, DagNode>) -> Result<Self> {
        let topological_order = Self::topological_sort(&nodes)?;
        Ok(Dag {
            nodes,
            topological_order,
        })
    }

    fn topological_sort(nodes: &HashMap<String, DagNode>) -> Result<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, HashSet<String>> = HashMap::new();

        // Initialize
        for (node_id, _) in nodes {
            in_degree.insert(node_id.clone(), 0);
            graph.insert(node_id.clone(), HashSet::new());
        }

        // Build graph and calculate in-degrees
        for (node_id, node) in nodes {
            for (_, connection) in &node.inputs {
                graph.get_mut(&connection.source_node)
                    .ok_or_else(|| DagError::NodeNotFound(connection.source_node.clone()))?
                    .insert(node_id.clone());
                *in_degree.get_mut(node_id).unwrap() += 1;
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut result: Vec<String> = Vec::new();

        for (node_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node_id.clone());
            }
        }

        while let Some(node_id) = queue.pop_front() {
            result.push(node_id.clone());

            if let Some(neighbors) = graph.get(&node_id) {
                for neighbor in neighbors {
                    let degree = in_degree.get_mut(neighbor).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        if result.len() != nodes.len() {
            return Err(DagError::CircularDependency.into());
        }

        Ok(result)
    }

    pub fn execute(&self) -> Result<HashMap<String, HashMap<String, Value>>> {
        let mut node_outputs: HashMap<String, HashMap<String, Value>> = HashMap::new();

        for node_id in &self.topological_order {
            let node = self.nodes.get(node_id)
                .ok_or_else(|| DagError::NodeNotFound(node_id.clone()))?;

            // Collect inputs
            let mut inputs = HashMap::new();
            for (input_name, connection) in &node.inputs {
                let source_outputs = node_outputs.get(&connection.source_node)
                    .ok_or_else(|| DagError::NodeNotFound(connection.source_node.clone()))?;
                
                let value = source_outputs.get(&connection.source_output)
                    .ok_or_else(|| DagError::InvalidInput(
                        format!("Output '{}' not found in node '{}'", 
                                connection.source_output, connection.source_node)))?
                    .clone();
                
                inputs.insert(input_name.clone(), value);
            }

            // Execute node
            let outputs = node.node.compute(inputs)?;
            node_outputs.insert(node_id.clone(), outputs);
        }

        Ok(node_outputs)
    }

    pub fn get_node_output(&self, node_id: &str, output_name: &str) -> Result<Value> {
        let results = self.execute()?;
        let node_outputs = results.get(node_id)
            .ok_or_else(|| DagError::NodeNotFound(node_id.to_string()))?;
        
        node_outputs.get(output_name)
            .cloned()
            .ok_or_else(|| DagError::InvalidInput(
                format!("Output '{}' not found in node '{}'", output_name, node_id)).into())
    }
}

pub struct DagBuilder {
    nodes: HashMap<String, DagNode>,
    registry: Arc<NodeRegistry>,
}

impl DagBuilder {
    pub fn new(registry: Arc<NodeRegistry>) -> Self {
        DagBuilder {
            nodes: HashMap::new(),
            registry,
        }
    }

    pub fn add_node(&mut self, id: String, node_type: &str, params: HashMap<String, Value>) -> Result<&mut Self> {
        let node = self.registry.create(node_type, params)?;
        self.nodes.insert(id.clone(), DagNode {
            id,
            node,
            inputs: HashMap::new(),
        });
        Ok(self)
    }

    pub fn connect(&mut self, 
                   from_node: &str, from_output: &str,
                   to_node: &str, to_input: &str) -> Result<&mut Self> {
        let dag_node = self.nodes.get_mut(to_node)
            .ok_or_else(|| DagError::NodeNotFound(to_node.to_string()))?;
        
        dag_node.inputs.insert(to_input.to_string(), Connection {
            source_node: from_node.to_string(),
            source_output: from_output.to_string(),
        });
        
        Ok(self)
    }

    pub fn build(self) -> Result<Dag> {
        Dag::new(self.nodes)
    }
}