#!/usr/bin/env python3
"""
Example of using the high-performance streaming DAG from Python.

This demonstrates:
1. Loading a DAG from YAML
2. Building a DAG programmatically
3. Streaming row-by-row evaluation with trigger-based outputs
"""

import sdag
import random

def example_yaml_loading():
    """Load and run the streaming example DAG from YAML"""
    print("=== YAML Loading Example ===")
    
    # Load the DAG from YAML file
    engine = sdag.PyEngine.from_yaml_file("streaming_example.yaml")
    
    # Simulate streaming price data
    print("\nStreaming price data...")
    for i in range(10):
        price_a = 40.0 + random.uniform(-5, 5)
        price_b = 45.0 + random.uniform(-5, 5)
        
        # Evaluate one step
        outputs = engine.evaluate_step([price_a, price_b])
        
        print(f"Row {i}: price_a={price_a:.2f}, price_b={price_b:.2f}")
        
        if outputs is not None:
            print(f"  -> TRIGGER FIRED! sum={outputs[0]:.2f}, adjusted={outputs[1]:.2f}")
        else:
            print(f"  -> No trigger (sum < 100)")

def example_programmatic_dag():
    """Build a DAG programmatically"""
    print("\n=== Programmatic DAG Example ===")
    
    # Build a simple DAG: trigger when (a * b) > 50
    builder = sdag.PyDagBuilder()
    
    # Add input nodes
    builder.add_input("a", 0)
    builder.add_input("b", 1)
    
    # Add computation nodes
    builder.add_multiply("product", "a", "b")
    builder.add_constant("threshold", 50.0)
    builder.add_comparison("trigger", "product", "threshold", "GreaterThan")
    
    # Set trigger and outputs
    builder.set_trigger("trigger")
    builder.set_outputs(["product"])
    
    # Build the engine
    engine = builder.build()
    
    # Test with some values
    print("\nTesting trigger when a * b > 50:")
    test_cases = [
        (5.0, 8.0),   # 40, no trigger
        (6.0, 9.0),   # 54, trigger!
        (7.0, 7.0),   # 49, no trigger
        (8.0, 8.0),   # 64, trigger!
    ]
    
    for a, b in test_cases:
        outputs = engine.evaluate_step([a, b])
        product = a * b
        print(f"  a={a}, b={b}, product={product}")
        if outputs is not None:
            print(f"    -> TRIGGER FIRED! Output: {outputs[0]}")
        else:
            print(f"    -> No trigger")

def example_streaming_batch():
    """Process a batch of streaming data"""
    print("\n=== Batch Streaming Example ===")
    
    # Create a simple threshold DAG
    builder = sdag.PyDagBuilder()
    builder.add_input("value", 0)
    builder.add_constant("threshold", 10.0)
    builder.add_comparison("above_threshold", "value", "threshold", "GreaterThan")
    builder.set_trigger("above_threshold")
    builder.set_outputs(["value"])
    
    engine = builder.build()
    
    # Generate streaming data
    streaming_data = [
        [5.0],   # Below threshold
        [8.0],   # Below threshold
        [12.0],  # Above threshold!
        [15.0],  # Above threshold!
        [9.0],   # Below threshold
        [11.0],  # Above threshold!
    ]
    
    # Process the stream
    outputs = engine.stream(streaming_data)
    
    print(f"Processed {len(streaming_data)} rows")
    print(f"Trigger fired {len(outputs)} times:")
    for i, output in enumerate(outputs):
        print(f"  Output {i}: {output}")

def example_yaml_inspection():
    """Show how to inspect and save DAG structure"""
    print("\n=== DAG YAML Inspection ===")
    
    # Build a DAG
    builder = sdag.PyDagBuilder()
    builder.add_input("x", 0)
    builder.add_input("y", 1)
    builder.add_add("sum", "x", "y")
    builder.add_constant("factor", 2.0)
    builder.add_multiply("result", "sum", "factor")
    builder.set_trigger("sum")
    builder.set_outputs(["result"])
    
    # Get YAML representation
    yaml_str = builder.to_yaml()
    print("Generated YAML:")
    print(yaml_str)
    
    # You could save this to a file for pure Rust execution
    with open("generated_dag.yaml", "w") as f:
        f.write(yaml_str)
    print("\nSaved to generated_dag.yaml")

if __name__ == "__main__":
    # Run all examples
    example_yaml_loading()
    example_programmatic_dag()
    example_streaming_batch()
    example_yaml_inspection()
    
    print("\n=== Running DAG from CLI ===")
    # Demonstrate CLI functionality
    sdag.run_dag_cli(["streaming_example.yaml", "55.0", "60.0"])