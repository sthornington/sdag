#!/usr/bin/env python3
"""
Examples of using the high-performance streaming DAG from Python.

This demonstrates two approaches:
1. Object-oriented API - Build DAGs using Python objects (type-safe, recommended)
2. Direct YAML loading - Load pre-built DAGs from YAML files
"""

import sdag
import random

def example_object_oriented_api():
    """Build a DAG using the object-oriented API (recommended)"""
    print("=== Object-Oriented API Example ===")
    
    # Create a graph
    graph = sdag.Graph()
    
    # Create nodes - no string references!
    price_a = sdag.InputNode(0)
    price_b = sdag.InputNode(1)
    threshold = sdag.ConstantNode(100.0)
    
    # Compose operations using actual objects
    total = sdag.AddNode(price_a, price_b)
    alert = sdag.ComparisonNode(total, threshold, "GreaterThan")
    
    # Additional calculation
    multiplier = sdag.ConstantNode(1.5)
    adjusted = sdag.MultiplyNode(total, multiplier)
    
    # Add all nodes to graph
    for node in [price_a, price_b, threshold, total, alert, multiplier, adjusted]:
        graph.add_node(node)
    
    # Configure trigger and outputs
    graph.set_trigger(alert)
    graph.add_output(total)
    graph.add_output(adjusted)
    
    # Build engine
    engine = graph.build_engine()
    
    # Stream data
    print("\nStreaming price data...")
    for i in range(5):
        a = 40.0 + random.uniform(-5, 5)
        b = 45.0 + random.uniform(-5, 5)
        
        outputs = engine.evaluate_step([a, b])
        print(f"Row {i}: price_a={a:.2f}, price_b={b:.2f}")
        
        if outputs is not None:
            print(f"  -> ALERT! total={outputs[0]:.2f}, adjusted={outputs[1]:.2f}")
        else:
            print(f"  -> No alert (sum < 100)")

def example_yaml_loading():
    """Load and run a DAG from YAML"""
    print("\n=== YAML Loading Example ===")
    
    # Load from YAML file
    engine = sdag.PyEngine.from_yaml_file("streaming_example.yaml")
    
    # Process some data
    print("\nProcessing with YAML-loaded DAG:")
    test_cases = [
        (45.0, 45.0),  # 90 < 100
        (55.0, 55.0),  # 110 > 100
        (40.0, 40.0),  # 80 < 100
    ]
    
    for a, b in test_cases:
        outputs = engine.evaluate_step([a, b])
        print(f"Input: ({a}, {b})")
        if outputs:
            print(f"  -> Triggered! Outputs: {outputs}")
        else:
            print(f"  -> No trigger")

def example_save_to_yaml():
    """Build a DAG with objects and save to YAML"""
    print("\n=== Save to YAML Example ===")
    
    graph = sdag.Graph()
    
    # Build a simple monitoring DAG
    sensor1 = sdag.InputNode(0)
    sensor2 = sdag.InputNode(1)
    sensor3 = sdag.InputNode(2)
    
    # Average the sensors
    total = sdag.SumNode([sensor1, sensor2, sensor3])
    count = sdag.ConstantNode(3.0)
    average = sdag.MultiplyNode(total, sdag.ConstantNode(1.0/3.0))
    
    # Alert if average > threshold
    threshold = sdag.ConstantNode(25.0)
    alert = sdag.ComparisonNode(average, threshold, "GreaterThan")
    
    # Add nodes
    nodes = [sensor1, sensor2, sensor3, total, count, average, 
             sdag.ConstantNode(1.0/3.0), threshold, alert]
    for node in nodes:
        graph.add_node(node)
    
    graph.set_trigger(alert)
    graph.add_output(average)
    
    # Save to YAML
    yaml_str = graph.to_yaml()
    print("Generated YAML:")
    print(yaml_str)
    
    with open("sensor_monitor.yaml", "w") as f:
        f.write(yaml_str)
    print("\nSaved to sensor_monitor.yaml")

def example_type_safety():
    """Demonstrate type safety benefits of object API"""
    print("\n=== Type Safety Benefits ===")
    
    # With objects, errors are caught immediately:
    input1 = sdag.InputNode(0)
    input2 = sdag.InputNode(1)
    
    # This works - proper objects
    sum_node = sdag.AddNode(input1, input2)
    print(f"✓ Created sum node: {sum_node.node_id}")
    
    # This would fail at Python level (uncomment to see):
    # bad_node = sdag.AddNode(input1, "input2")  # TypeError!
    
    # With string-based builders, errors only show at runtime:
    # builder.add_add("sum", "inpt1", "input2")  # Typo undetected until execution!
    
    print("✓ Object API prevents typos and type errors at construction time")

if __name__ == "__main__":
    example_object_oriented_api()
    example_yaml_loading()
    example_save_to_yaml()
    example_type_safety()