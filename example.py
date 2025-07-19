#!/usr/bin/env python3
import sdag

def main():
    # Create a DAG
    dag = sdag.PyDag()
    
    # Add constant nodes
    dag.add_node("x", "Constant", {"value": 10.0})
    dag.add_node("y", "Constant", {"value": 20.0})
    dag.add_node("z", "Constant", {"value": 2.0})
    
    # Add operation nodes
    dag.add_node("add", "Add")
    dag.add_node("multiply", "Multiply")
    
    # Connect nodes: (x + y) * z
    dag.connect("x", "value", "add", "a")
    dag.connect("y", "value", "add", "b")
    dag.connect("add", "result", "multiply", "a")
    dag.connect("z", "value", "multiply", "b")
    
    # Execute the DAG
    print("Executing DAG: (10 + 20) * 2")
    results = dag.execute()
    
    # Print results
    print("\nResults:")
    for node_id, outputs in results.items():
        print(f"  {node_id}: {outputs}")
    
    # Save to YAML
    yaml_file = "example_dag.yaml"
    dag.save_yaml(yaml_file)
    print(f"\nSaved DAG to {yaml_file}")
    
    # Load from YAML and execute
    print(f"\nLoading DAG from {yaml_file}")
    loaded_dag = sdag.load_dag_from_yaml(open(yaml_file).read())
    loaded_results = loaded_dag.execute()
    
    print("\nResults from loaded DAG:")
    for node_id, outputs in loaded_results.items():
        print(f"  {node_id}: {outputs}")

if __name__ == "__main__":
    main()