#[cfg(test)]
mod tests {
    use crate::{Engine, NodeOp, ComparisonOp};
    
    #[test]
    fn test_simple_streaming_dag() {
        // Create a simple DAG: (a + b) * 2
        let nodes = vec![
            NodeOp::Input { input_index: 0 }, // 0: a
            NodeOp::Input { input_index: 1 }, // 1: b
            NodeOp::Add { a: 0, b: 1 },       // 2: a + b
            NodeOp::Constant(2.0),             // 3: constant 2
            NodeOp::Multiply { a: 2, b: 3 },  // 4: (a + b) * 2
        ];
        
        let mut engine = Engine::new(nodes);
        engine.set_trigger(2); // Trigger on sum change
        engine.set_outputs(vec![4]); // Output the product
        
        // First row: a=1, b=2
        let outputs = engine.evaluate_step(&[1.0, 2.0]);
        assert!(outputs.is_some());
        assert_eq!(outputs.unwrap(), vec![6.0]); // (1+2)*2 = 6
        
        // Second row: same values, no trigger
        let outputs = engine.evaluate_step(&[1.0, 2.0]);
        assert!(outputs.is_none());
        
        // Third row: a=2, b=3, trigger fires
        let outputs = engine.evaluate_step(&[2.0, 3.0]);
        assert!(outputs.is_some());
        assert_eq!(outputs.unwrap(), vec![10.0]); // (2+3)*2 = 10
    }
    
    #[test]
    fn test_comparison_trigger() {
        // Create DAG: trigger when a + b > 5
        let nodes = vec![
            NodeOp::Input { input_index: 0 }, // 0: a
            NodeOp::Input { input_index: 1 }, // 1: b
            NodeOp::Add { a: 0, b: 1 },       // 2: a + b
            NodeOp::Constant(5.0),             // 3: threshold
            NodeOp::Comparison { a: 2, b: 3, op: ComparisonOp::GreaterThan }, // 4: sum > 5
        ];
        
        let mut engine = Engine::new(nodes);
        engine.set_trigger(4); // Trigger on comparison
        engine.set_outputs(vec![2]); // Output the sum
        
        // First row: 2 + 2 = 4, not > 5, but first run always triggers
        let outputs = engine.evaluate_step(&[2.0, 2.0]);
        assert!(outputs.is_some()); // First run always triggers
        
        // Second row: 3 + 2 = 5, not > 5, no trigger
        let outputs = engine.evaluate_step(&[3.0, 2.0]);
        assert!(outputs.is_none());
        
        // Third row: 3 + 3 = 6 > 5, trigger fires
        let outputs = engine.evaluate_step(&[3.0, 3.0]);
        assert!(outputs.is_some());
        assert_eq!(outputs.unwrap(), vec![6.0]);
    }
    
    #[test]
    fn test_incremental_computation() {
        // Create DAG with multiple paths
        let nodes = vec![
            NodeOp::Input { input_index: 0 }, // 0: a
            NodeOp::Input { input_index: 1 }, // 1: b
            NodeOp::Constant(10.0),            // 2: constant
            NodeOp::Add { a: 0, b: 2 },       // 3: a + 10
            NodeOp::Add { a: 1, b: 2 },       // 4: b + 10
            NodeOp::Multiply { a: 3, b: 4 },  // 5: (a+10) * (b+10)
        ];
        
        let mut engine = Engine::new(nodes);
        
        // First evaluation
        engine.evaluate_step(&[1.0, 2.0]);
        assert_eq!(engine.get_value(5), 11.0 * 12.0); // (1+10) * (2+10) = 132
        
        // Change only a, b path should not recompute
        engine.evaluate_step(&[2.0, 2.0]);
        assert_eq!(engine.get_value(3), 12.0); // a + 10 = 12
        assert_eq!(engine.get_value(4), 12.0); // b + 10 = 12 (not recomputed)
        assert_eq!(engine.get_value(5), 12.0 * 12.0); // 144
    }
    
    #[test]
    fn test_yaml_loading() {
        let yaml = r#"
nodes:
  - id: x
    type: Input
    params:
      input_index: 0
  - id: y
    type: Input
    params:
      input_index: 1
  - id: sum
    type: Add
    params:
      inputs: ["x", "y"]
trigger: sum
outputs: ["sum"]
"#;
        
        let mut engine = crate::engine::from_yaml(yaml).unwrap();
        let outputs = engine.evaluate_step(&[5.0, 7.0]);
        assert!(outputs.is_some());
        assert_eq!(outputs.unwrap(), vec![12.0]);
    }
}