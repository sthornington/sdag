use std::collections::HashMap;
use anyhow::Result;
use crate::{DagError, NodeOp};

pub type NodeId = usize;

/// High-performance streaming DAG engine
/// 
/// The engine stores all node data in dense arrays for cache-friendly access:
/// - nodes: Vec<NodeOp> - the operation type and input indices  
/// - values: Vec<f64> - current output value for each node
/// - prev_values: Vec<f64> - previous output values for change detection
/// - changed: Vec<bool> - dirty flags indicating which nodes changed this step
pub struct Engine {
    // Static graph structure
    nodes: Vec<NodeOp>,
    
    // Node state (parallel arrays indexed by NodeId)
    values: Vec<f64>,
    prev_values: Vec<f64>,
    changed: Vec<bool>,
    
    // Special node sets
    input_nodes: Vec<(NodeId, usize)>, // (node_id, input_index)
    trigger_node: Option<NodeId>,
    output_nodes: Vec<NodeId>,
    
    // First run flag
    first_run: bool,
}

impl Engine {
    /// Create a new engine from nodes in topological order
    pub fn new(nodes: Vec<NodeOp>) -> Self {
        let n = nodes.len();
        
        // Find input nodes
        let mut input_nodes = Vec::new();
        for (i, node) in nodes.iter().enumerate() {
            if let NodeOp::Input { input_index } = node {
                input_nodes.push((i, *input_index));
            }
        }
        
        Engine {
            nodes,
            values: vec![0.0; n],
            prev_values: vec![0.0; n],
            changed: vec![false; n],
            input_nodes,
            trigger_node: None,
            output_nodes: Vec::new(),
            first_run: true,
        }
    }
    
    /// Set the trigger node that controls output emission
    pub fn set_trigger(&mut self, trigger: NodeId) {
        self.trigger_node = Some(trigger);
    }
    
    /// Set the output nodes to collect when trigger fires
    pub fn set_outputs(&mut self, outputs: Vec<NodeId>) {
        self.output_nodes = outputs;
    }
    
    /// Evaluate the DAG for one row of input values
    /// Returns Some(outputs) if trigger fired, None otherwise
    pub fn evaluate_step(&mut self, input_values: &[f64]) -> Option<Vec<f64>> {
        let n = self.nodes.len();
        
        if self.first_run {
            // First run: mark everything dirty and compute all values
            self.changed.fill(true);
            
            // Set input values
            for &(node_id, input_idx) in &self.input_nodes {
                self.values[node_id] = input_values[input_idx];
            }
            
            // Compute all nodes in topological order
            for i in 0..n {
                if !matches!(self.nodes[i], NodeOp::Input { .. }) {
                    self.compute_node(i);
                }
            }
            
            self.first_run = false;
        } else {
            // Incremental update
            self.values.copy_from_slice(&self.prev_values);
            self.changed.fill(false);
            
            // Update input nodes and mark dirty if changed
            for &(node_id, input_idx) in &self.input_nodes {
                let new_val = input_values[input_idx];
                let old_val = self.prev_values[node_id];
                
                if (new_val - old_val).abs() > f64::EPSILON {
                    self.values[node_id] = new_val;
                    self.changed[node_id] = true;
                }
            }
            
            // Single pass evaluation in topological order
            for i in 0..n {
                match &self.nodes[i] {
                    NodeOp::Input { .. } => {
                        // Already handled above
                    }
                    NodeOp::Constant(_) => {
                        // Constants never change after first run
                    }
                    _ => {
                        // Check if any inputs changed
                        let inputs_changed = self.check_inputs_changed(i);
                        
                        if inputs_changed {
                            let old_val = self.prev_values[i];
                            self.compute_node(i);
                            let new_val = self.values[i];
                            
                            // Node decides if it changed enough to propagate
                            if (new_val - old_val).abs() > f64::EPSILON {
                                self.changed[i] = true;
                            }
                        }
                    }
                }
            }
        }
        
        // Save current values for next iteration
        self.prev_values.copy_from_slice(&self.values);
        
        // Check trigger and emit outputs if fired
        if let Some(trigger) = self.trigger_node {
            if self.changed[trigger] {
                let outputs: Vec<f64> = self.output_nodes.iter()
                    .map(|&id| self.values[id])
                    .collect();
                return Some(outputs);
            }
        }
        
        None
    }
    
    /// Check if any inputs to a node changed
    fn check_inputs_changed(&self, node_id: NodeId) -> bool {
        match &self.nodes[node_id] {
            NodeOp::Add { a, b } | NodeOp::Multiply { a, b } => {
                self.changed[*a] || self.changed[*b]
            }
            NodeOp::Sum { inputs } => {
                inputs.iter().any(|&i| self.changed[i])
            }
            NodeOp::ConstantProduct { input, .. } => {
                self.changed[*input]
            }
            NodeOp::Comparison { a, b, .. } => {
                self.changed[*a] || self.changed[*b]
            }
            _ => false,
        }
    }
    
    /// Compute a node's value (assumes inputs are already computed)
    fn compute_node(&mut self, i: NodeId) {
        // SAFETY: This is the hot path. We know i < nodes.len() from the caller.
        // All input indices are validated during graph construction.
        unsafe {
            let node = self.nodes.get_unchecked(i);
            let result = match node {
                NodeOp::Constant(val) => *val,
                NodeOp::Input { .. } => *self.values.get_unchecked(i), // Already set
                NodeOp::Add { a, b } => {
                    *self.values.get_unchecked(*a) + *self.values.get_unchecked(*b)
                }
                NodeOp::Multiply { a, b } => {
                    *self.values.get_unchecked(*a) * *self.values.get_unchecked(*b)
                }
                NodeOp::Sum { inputs } => {
                    inputs.iter().map(|&idx| *self.values.get_unchecked(idx)).sum()
                }
                NodeOp::ConstantProduct { input, factor } => {
                    *self.values.get_unchecked(*input) * factor
                }
                NodeOp::Comparison { a, b, op } => {
                    let va = *self.values.get_unchecked(*a);
                    let vb = *self.values.get_unchecked(*b);
                    match op {
                        crate::ComparisonOp::GreaterThan => if va > vb { 1.0 } else { 0.0 },
                        crate::ComparisonOp::LessThan => if va < vb { 1.0 } else { 0.0 },
                        crate::ComparisonOp::Equal => if (va - vb).abs() < f64::EPSILON { 1.0 } else { 0.0 },
                    }
                }
            };
            *self.values.get_unchecked_mut(i) = result;
        }
    }
    
    /// Get the current value of a node
    pub fn get_value(&self, node_id: NodeId) -> f64 {
        self.values[node_id]
    }
    
    /// Get all current values
    pub fn get_all_values(&self) -> &[f64] {
        &self.values
    }
}

/// Build an engine from a YAML string
pub fn from_yaml(yaml_str: &str) -> Result<Engine> {
    let dag_yaml: crate::DagYaml = serde_yaml::from_str(yaml_str)?;
    
    // Map string IDs to indices
    let mut id_map = HashMap::new();
    for (i, node) in dag_yaml.nodes.iter().enumerate() {
        id_map.insert(node.id.clone(), i);
    }
    
    // Convert YAML nodes to NodeOp enum
    let mut nodes = Vec::with_capacity(dag_yaml.nodes.len());
    for node in &dag_yaml.nodes {
        let op = match node.node_type.as_str() {
            "Constant" => {
                let value = node.params.get("value")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| DagError::InvalidInput("Constant requires 'value' parameter".into()))?;
                NodeOp::Constant(value)
            }
            "Input" => {
                let input_index = node.params.get("input_index")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| DagError::InvalidInput("Input requires 'input_index' parameter".into()))?
                    as usize;
                NodeOp::Input { input_index }
            }
            "Add" => {
                let inputs = node.params.get("inputs")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| DagError::InvalidInput("Add requires 'inputs' array".into()))?;
                if inputs.len() != 2 {
                    return Err(DagError::InvalidInput("Add requires exactly 2 inputs".into()).into());
                }
                let a = inputs[0].as_str()
                    .and_then(|s| id_map.get(s))
                    .ok_or_else(|| DagError::NodeNotFound(inputs[0].to_string()))?;
                let b = inputs[1].as_str()
                    .and_then(|s| id_map.get(s))
                    .ok_or_else(|| DagError::NodeNotFound(inputs[1].to_string()))?;
                NodeOp::Add { a: *a, b: *b }
            }
            "Multiply" => {
                let inputs = node.params.get("inputs")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| DagError::InvalidInput("Multiply requires 'inputs' array".into()))?;
                if inputs.len() != 2 {
                    return Err(DagError::InvalidInput("Multiply requires exactly 2 inputs".into()).into());
                }
                let a = inputs[0].as_str()
                    .and_then(|s| id_map.get(s))
                    .ok_or_else(|| DagError::NodeNotFound(inputs[0].to_string()))?;
                let b = inputs[1].as_str()
                    .and_then(|s| id_map.get(s))
                    .ok_or_else(|| DagError::NodeNotFound(inputs[1].to_string()))?;
                NodeOp::Multiply { a: *a, b: *b }
            }
            "Comparison" => {
                let inputs = node.params.get("inputs")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| DagError::InvalidInput("Comparison requires 'inputs' array".into()))?;
                if inputs.len() != 2 {
                    return Err(DagError::InvalidInput("Comparison requires exactly 2 inputs".into()).into());
                }
                let a = inputs[0].as_str()
                    .and_then(|s| id_map.get(s))
                    .ok_or_else(|| DagError::NodeNotFound(inputs[0].to_string()))?;
                let b = inputs[1].as_str()
                    .and_then(|s| id_map.get(s))
                    .ok_or_else(|| DagError::NodeNotFound(inputs[1].to_string()))?;
                
                let op_str = node.params.get("op")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| DagError::InvalidInput("Comparison requires 'op' parameter".into()))?;
                
                let op = match op_str {
                    "GreaterThan" => crate::ComparisonOp::GreaterThan,
                    "LessThan" => crate::ComparisonOp::LessThan,
                    "Equal" => crate::ComparisonOp::Equal,
                    _ => return Err(DagError::InvalidInput(format!("Unknown comparison op: {}", op_str)).into()),
                };
                
                NodeOp::Comparison { a: *a, b: *b, op }
            }
            _ => return Err(DagError::InvalidInput(format!("Unknown node type: {}", node.node_type)).into()),
        };
        nodes.push(op);
    }
    
    // TODO: Verify topological order
    
    let mut engine = Engine::new(nodes);
    
    // Set trigger and outputs if specified
    if let Some(trigger_id) = dag_yaml.trigger {
        let trigger_idx = id_map.get(&trigger_id)
            .ok_or_else(|| DagError::NodeNotFound(trigger_id))?;
        engine.set_trigger(*trigger_idx);
    }
    
    if let Some(output_ids) = dag_yaml.outputs {
        let output_indices: Result<Vec<_>> = output_ids.iter()
            .map(|id| id_map.get(id)
                .copied()
                .ok_or_else(|| DagError::NodeNotFound(id.clone()).into()))
            .collect();
        engine.set_outputs(output_indices?);
    }
    
    Ok(engine)
}