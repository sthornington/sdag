#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::collections::HashMap;
    use crate::{DagBuilder, NodeRegistry, Value, DagYaml, yaml::{NodeYaml, ConnectionYaml}};

    #[test]
    fn test_constant_node() {
        let registry = Arc::new(NodeRegistry::new());
        let mut params = HashMap::new();
        params.insert("value".to_string(), Value::Float(42.0));
        
        let node = registry.create("Constant", params).unwrap();
        let outputs = node.compute(HashMap::new()).unwrap();
        
        assert_eq!(outputs.get("value").unwrap().as_f64().unwrap(), 42.0);
    }

    #[test]
    fn test_add_node() {
        let registry = Arc::new(NodeRegistry::new());
        let node = registry.create("Add", HashMap::new()).unwrap();
        
        let mut inputs = HashMap::new();
        inputs.insert("a".to_string(), Value::Float(5.0));
        inputs.insert("b".to_string(), Value::Float(3.0));
        
        let outputs = node.compute(inputs).unwrap();
        assert_eq!(outputs.get("result").unwrap().as_f64().unwrap(), 8.0);
    }

    #[test]
    fn test_multiply_node() {
        let registry = Arc::new(NodeRegistry::new());
        let node = registry.create("Multiply", HashMap::new()).unwrap();
        
        let mut inputs = HashMap::new();
        inputs.insert("a".to_string(), Value::Float(4.0));
        inputs.insert("b".to_string(), Value::Float(7.0));
        
        let outputs = node.compute(inputs).unwrap();
        assert_eq!(outputs.get("result").unwrap().as_f64().unwrap(), 28.0);
    }

    #[test]
    fn test_simple_dag() {
        let registry = Arc::new(NodeRegistry::new());
        let mut builder = DagBuilder::new(registry.clone());
        
        // Create constants
        let mut params1 = HashMap::new();
        params1.insert("value".to_string(), Value::Float(10.0));
        builder.add_node("const1".to_string(), "Constant", params1).unwrap();
        
        let mut params2 = HashMap::new();
        params2.insert("value".to_string(), Value::Float(20.0));
        builder.add_node("const2".to_string(), "Constant", params2).unwrap();
        
        // Add node
        builder.add_node("add".to_string(), "Add", HashMap::new()).unwrap();
        
        // Connect
        builder.connect("const1", "value", "add", "a").unwrap();
        builder.connect("const2", "value", "add", "b").unwrap();
        
        let dag = builder.build().unwrap();
        let result = dag.get_node_output("add", "result").unwrap();
        
        assert_eq!(result.as_f64().unwrap(), 30.0);
    }

    #[test]
    fn test_complex_dag() {
        let registry = Arc::new(NodeRegistry::new());
        let mut builder = DagBuilder::new(registry.clone());
        
        // Create constants
        let mut params1 = HashMap::new();
        params1.insert("value".to_string(), Value::Float(2.0));
        builder.add_node("x".to_string(), "Constant", params1).unwrap();
        
        let mut params2 = HashMap::new();
        params2.insert("value".to_string(), Value::Float(3.0));
        builder.add_node("y".to_string(), "Constant", params2).unwrap();
        
        let mut params3 = HashMap::new();
        params3.insert("value".to_string(), Value::Float(4.0));
        builder.add_node("z".to_string(), "Constant", params3).unwrap();
        
        // Create operations: (x + y) * z
        builder.add_node("add".to_string(), "Add", HashMap::new()).unwrap();
        builder.add_node("multiply".to_string(), "Multiply", HashMap::new()).unwrap();
        
        // Connect
        builder.connect("x", "value", "add", "a").unwrap();
        builder.connect("y", "value", "add", "b").unwrap();
        builder.connect("add", "result", "multiply", "a").unwrap();
        builder.connect("z", "value", "multiply", "b").unwrap();
        
        let dag = builder.build().unwrap();
        let result = dag.get_node_output("multiply", "result").unwrap();
        
        // (2 + 3) * 4 = 20
        assert_eq!(result.as_f64().unwrap(), 20.0);
    }

    #[test]
    fn test_yaml_serialization() {
        let dag_yaml = DagYaml {
            nodes: vec![
                NodeYaml {
                    id: "const1".to_string(),
                    node_type: "Constant".to_string(),
                    params: {
                        let mut params = HashMap::new();
                        params.insert("value".to_string(), Value::Float(5.0));
                        params
                    },
                },
                NodeYaml {
                    id: "const2".to_string(),
                    node_type: "Constant".to_string(),
                    params: {
                        let mut params = HashMap::new();
                        params.insert("value".to_string(), Value::Float(10.0));
                        params
                    },
                },
                NodeYaml {
                    id: "add".to_string(),
                    node_type: "Add".to_string(),
                    params: HashMap::new(),
                },
            ],
            connections: vec![
                ConnectionYaml {
                    from_node: "const1".to_string(),
                    from_output: "value".to_string(),
                    to_node: "add".to_string(),
                    to_input: "a".to_string(),
                },
                ConnectionYaml {
                    from_node: "const2".to_string(),
                    from_output: "value".to_string(),
                    to_node: "add".to_string(),
                    to_input: "b".to_string(),
                },
            ],
        };

        let yaml_str = dag_yaml.to_yaml().unwrap();
        let parsed = DagYaml::from_yaml(&yaml_str).unwrap();
        
        assert_eq!(parsed.nodes.len(), 3);
        assert_eq!(parsed.connections.len(), 2);
    }

    #[test]
    fn test_yaml_to_dag() {
        let yaml_str = r#"
nodes:
  - id: a
    node_type: Constant
    params:
      value: 7.0
  - id: b
    node_type: Constant
    params:
      value: 3.0
  - id: mult
    node_type: Multiply
    params: {}
connections:
  - from_node: a
    from_output: value
    to_node: mult
    to_input: a
  - from_node: b
    from_output: value
    to_node: mult
    to_input: b
"#;

        let registry = Arc::new(NodeRegistry::new());
        let dag = crate::yaml::load_dag_from_yaml(yaml_str, registry).unwrap();
        let result = dag.get_node_output("mult", "result").unwrap();
        
        assert_eq!(result.as_f64().unwrap(), 21.0);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let registry = Arc::new(NodeRegistry::new());
        let mut builder = DagBuilder::new(registry.clone());
        
        builder.add_node("a".to_string(), "Add", HashMap::new()).unwrap();
        builder.add_node("b".to_string(), "Add", HashMap::new()).unwrap();
        
        // Create circular dependency
        builder.connect("a", "result", "b", "a").unwrap();
        builder.connect("b", "result", "a", "a").unwrap();
        
        let result = builder.build();
        assert!(result.is_err());
    }
}