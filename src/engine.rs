use anyhow::Result;
use crate::NodeOp;

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
    
    /// Set which node acts as the trigger
    pub fn set_trigger(&mut self, node_id: NodeId) {
        self.trigger_node = Some(node_id);
    }
    
    /// Set which nodes to output when triggered
    pub fn set_outputs(&mut self, node_ids: Vec<NodeId>) {
        self.output_nodes = node_ids;
    }
    
    /// Process one step of streaming input
    /// Returns Some(outputs) if trigger fired, None otherwise
    pub fn evaluate_step(&mut self, input_values: &[f64]) -> Option<Vec<f64>> {
        // Clear changed flags
        self.changed.fill(false);
        
        // Update input nodes
        for &(node_id, input_index) in &self.input_nodes {
            if input_index < input_values.len() {
                let new_val = input_values[input_index];
                let old_val = self.values[node_id];
                
                if self.first_run || (new_val - old_val).abs() > f64::EPSILON {
                    self.values[node_id] = new_val;
                    self.changed[node_id] = true;
                }
            }
        }
        
        // Mark all nodes as changed on first run
        if self.first_run {
            self.changed.fill(true);
            self.first_run = false;
        }
        
        // Evaluate nodes in topological order (they're already sorted)
        // This is the hot path - optimize for CPU cache and branch prediction
        for i in 0..self.nodes.len() {
            if self.changed[i] {
                // Node is marked as changed, compute it
                self.compute_node(i);
            } else {
                // Check if any inputs changed
                match &self.nodes[i] {
                    NodeOp::Constant { .. } => continue, // Never changes after first run
                    NodeOp::Input { .. } => continue, // Already handled above
                    _ => {
                        if self.check_inputs_changed(i) {
                            let old_val = self.values[i];
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
            // Trigger fires when value is > 0 (truthy)
            if self.values[trigger] > 0.0 {
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
        let inputs = self.nodes[node_id].inputs();
        inputs.iter().any(|&idx| self.changed[idx])
    }
    
    /// Compute a single node's value
    /// SAFETY: This is the hot path. We use unsafe array access after bounds checking
    /// at construction time. All node indices are guaranteed valid.
    #[inline(always)]
    fn compute_node(&mut self, i: NodeId) {
        unsafe {
            let node = self.nodes.get_unchecked(i);
            let result = match node {
                NodeOp::Constant { value } => *value,
                NodeOp::Input { .. } => *self.values.get_unchecked(i), // Already set
                NodeOp::Add { inputs } => {
                    // Assuming exactly 2 inputs for Add
                    *self.values.get_unchecked(inputs[0]) + *self.values.get_unchecked(inputs[1])
                }
                NodeOp::Multiply { inputs } => {
                    // Assuming exactly 2 inputs for Multiply
                    *self.values.get_unchecked(inputs[0]) * *self.values.get_unchecked(inputs[1])
                }
                NodeOp::Sum { inputs } => {
                    inputs.iter().map(|&idx| *self.values.get_unchecked(idx)).sum()
                }
                NodeOp::ConstantProduct { inputs, factor } => {
                    *self.values.get_unchecked(inputs[0]) * factor
                }
                NodeOp::Comparison { inputs, op } => {
                    let va = *self.values.get_unchecked(inputs[0]);
                    let vb = *self.values.get_unchecked(inputs[1]);
                    match op {
                        crate::ComparisonOp::GreaterThan => if va > vb { 1.0 } else { 0.0 },
                        crate::ComparisonOp::LessThan => if va < vb { 1.0 } else { 0.0 },
                        crate::ComparisonOp::Equal => if (va - vb).abs() < f64::EPSILON { 1.0 } else { 0.0 },
                    }
                }
                NodeOp::Pow { inputs } => {
                    let base = *self.values.get_unchecked(inputs[0]);
                    let exp = *self.values.get_unchecked(inputs[1]);
                    base.powf(exp)
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

/// Simplified YAML format using indices directly
#[derive(Debug, serde::Deserialize)]
struct DagYamlSimple {
    nodes: Vec<NodeOp>,
    trigger: Option<usize>,
    outputs: Option<Vec<usize>>,
}

/// Build an engine from a YAML string
pub fn from_yaml(yaml_str: &str) -> Result<Engine> {
    // Parse YAML directly - no string resolution needed!
    let dag: DagYamlSimple = serde_yaml::from_str(yaml_str)?;
    
    let mut engine = Engine::new(dag.nodes);
    
    // Set trigger and outputs (already indices)
    if let Some(trigger) = dag.trigger {
        engine.set_trigger(trigger);
    }
    
    if let Some(outputs) = dag.outputs {
        engine.set_outputs(outputs);
    }
    
    Ok(engine)
}