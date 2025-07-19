use std::collections::HashMap;
use crate::arena::NodeId;
use crate::engine_traits::{Engine, EvalNode};

/// Basic topological evaluation engine
pub struct TopologicalEngine;

impl Engine for TopologicalEngine {
    fn name(&self) -> &str {
        "topological"
    }
    
    fn evaluate(
        &self,
        nodes: &[Box<dyn EvalNode>],
        root: NodeId,
        outputs: &[NodeId],
        rows: Vec<HashMap<String, f64>>,
    ) -> Vec<HashMap<String, f64>> {
        let mut results = Vec::new();
        
        for row in rows {
            // Evaluate all nodes in topological order (arena is already sorted)
            let mut values = vec![0.0; nodes.len()];
            
            for (i, node) in nodes.iter().enumerate() {
                values[i] = node.eval_arena(&values, &row);
            }
            
            // Build output record
            let mut record = HashMap::new();
            record.insert("trigger".to_string(), values[root]);
            
            for &output_id in outputs {
                record.insert(format!("output{}", output_id), values[output_id]);
            }
            
            results.push(record);
        }
        
        results
    }
}

/// Lazy evaluation engine - only evaluates needed nodes
pub struct LazyEngine;

impl LazyEngine {
    fn evaluate_node(
        &self,
        node_id: NodeId,
        nodes: &[Box<dyn EvalNode>],
        values: &mut Vec<Option<f64>>,
        row: &HashMap<String, f64>,
    ) -> f64 {
        if let Some(value) = values[node_id] {
            return value;
        }
        
        // Convert values to array for eval_arena
        let mut value_array = vec![0.0; nodes.len()];
        for (i, v) in values.iter().enumerate() {
            if let Some(val) = v {
                value_array[i] = *val;
            }
        }
        
        let result = nodes[node_id].eval_arena(&value_array, row);
        values[node_id] = Some(result);
        result
    }
}

impl Engine for LazyEngine {
    fn name(&self) -> &str {
        "lazy"
    }
    
    fn evaluate(
        &self,
        nodes: &[Box<dyn EvalNode>],
        root: NodeId,
        outputs: &[NodeId],
        rows: Vec<HashMap<String, f64>>,
    ) -> Vec<HashMap<String, f64>> {
        let mut results = Vec::new();
        
        for row in rows {
            let mut values = vec![None; nodes.len()];
            
            // Evaluate root
            let root_value = self.evaluate_node(root, nodes, &mut values, &row);
            
            // Evaluate outputs
            let mut record = HashMap::new();
            record.insert("trigger".to_string(), root_value);
            
            for &output_id in outputs {
                let output_value = self.evaluate_node(output_id, nodes, &mut values, &row);
                record.insert(format!("output{}", output_id), output_value);
            }
            
            results.push(record);
        }
        
        results
    }
}

/// Parallel evaluation engine using rayon
#[cfg(feature = "parallel")]
pub struct ParallelEngine;

#[cfg(feature = "parallel")]
impl Engine for ParallelEngine {
    fn name(&self) -> &str {
        "parallel"
    }
    
    fn evaluate(
        &self,
        nodes: &[Box<dyn EvalNode>],
        root: NodeId,
        outputs: &[NodeId],
        rows: Vec<HashMap<String, f64>>,
    ) -> Vec<HashMap<String, f64>> {
        use rayon::prelude::*;
        
        rows.par_iter()
            .map(|row| {
                let mut values = vec![0.0; nodes.len()];
                
                for (i, node) in nodes.iter().enumerate() {
                    values[i] = node.eval_arena(&values, row);
                }
                
                let mut record = HashMap::new();
                record.insert("trigger".to_string(), values[root]);
                
                for &output_id in outputs {
                    record.insert(format!("output{}", output_id), values[output_id]);
                }
                
                record
            })
            .collect()
    }
}