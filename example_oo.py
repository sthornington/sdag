#!/usr/bin/env python3
"""
Example of using the object-oriented API for building DAGs.

This demonstrates building DAGs using Python objects instead of string references,
which provides type safety and prevents typos.
"""

import sdag

def example_object_composition():
    """Build a DAG using object composition"""
    print("=== Object-Oriented DAG Building ===")
    
    # Create a graph to collect nodes
    graph = sdag.Graph()
    
    # Create input nodes
    price_a = sdag.InputNode(0)  # Input index 0
    price_b = sdag.InputNode(1)  # Input index 1
    
    # Create a constant threshold
    threshold = sdag.ConstantNode(100.0)
    
    # Compose operations using objects (not string names!)
    total_price = sdag.AddNode(price_a, price_b)
    
    # Create comparison trigger
    price_alert = sdag.ComparisonNode(total_price, threshold, "GreaterThan")
    
    # Create adjusted price calculation
    adjustment_factor = sdag.ConstantNode(1.5)
    adjusted_price = sdag.MultiplyNode(total_price, adjustment_factor)
    
    # Add all nodes to the graph
    graph.add_node(price_a)
    graph.add_node(price_b)
    graph.add_node(threshold)
    graph.add_node(total_price)
    graph.add_node(price_alert)
    graph.add_node(adjustment_factor)
    graph.add_node(adjusted_price)
    
    # Set trigger and outputs
    graph.set_trigger(price_alert)
    graph.add_output(total_price)
    graph.add_output(adjusted_price)
    
    # Build the engine
    engine = graph.build_engine()
    
    # Test with streaming data
    print("\nStreaming price data:")
    test_data = [
        (45.0, 45.0),   # 90 < 100, no trigger
        (55.0, 50.0),   # 105 > 100, trigger!
        (40.0, 50.0),   # 90 < 100, no trigger  
        (60.0, 60.0),   # 120 > 100, trigger!
    ]
    
    for i, (a, b) in enumerate(test_data):
        outputs = engine.evaluate_step([a, b])
        print(f"Row {i}: price_a={a}, price_b={b}")
        if outputs is not None:
            print(f"  -> TRIGGER! total={outputs[0]}, adjusted={outputs[1]}")
        else:
            print(f"  -> No trigger")

def example_complex_dag():
    """Build a more complex DAG with multiple paths"""
    print("\n=== Complex DAG Example ===")
    
    graph = sdag.Graph()
    
    # Multiple inputs
    inputs = [sdag.InputNode(i) for i in range(4)]
    
    # Create multiple computation paths
    sum_01 = sdag.AddNode(inputs[0], inputs[1])
    sum_23 = sdag.AddNode(inputs[2], inputs[3])
    
    # Combine the sums
    total_sum = sdag.AddNode(sum_01, sum_23)
    
    # Create a threshold check
    threshold = sdag.ConstantNode(10.0)
    trigger = sdag.ComparisonNode(total_sum, threshold, "GreaterThan")
    
    # Add nodes to graph
    for inp in inputs:
        graph.add_node(inp)
    graph.add_node(sum_01)
    graph.add_node(sum_23)
    graph.add_node(total_sum)
    graph.add_node(threshold)
    graph.add_node(trigger)
    
    graph.set_trigger(trigger)
    graph.add_output(total_sum)
    
    # Save to YAML
    yaml_str = graph.to_yaml()
    print("Generated YAML:")
    print(yaml_str)
    
    # Build and test
    engine = graph.build_engine()
    outputs = engine.evaluate_step([2.0, 3.0, 4.0, 5.0])
    if outputs:
        print(f"\nResult: {outputs[0]} (sum of all inputs)")

def example_sum_node():
    """Example using SumNode for multiple inputs"""
    print("\n=== Sum Node Example ===")
    
    graph = sdag.Graph()
    
    # Create many inputs
    inputs = [sdag.InputNode(i) for i in range(5)]
    
    # Sum them all at once
    total = sdag.SumNode(inputs)
    
    # Create trigger
    threshold = sdag.ConstantNode(15.0)
    trigger = sdag.ComparisonNode(total, threshold, "GreaterThan")
    
    # Build graph
    for inp in inputs:
        graph.add_node(inp)
    graph.add_node(total)
    graph.add_node(threshold)
    graph.add_node(trigger)
    
    graph.set_trigger(trigger)
    graph.add_output(total)
    
    engine = graph.build_engine()
    
    # Test
    print("Testing sum of 5 inputs:")
    test_cases = [
        [1, 2, 3, 4, 5],    # sum=15, trigger at threshold
        [2, 2, 2, 2, 2],    # sum=10, no trigger
        [4, 4, 4, 4, 4],    # sum=20, trigger!
    ]
    
    for inputs in test_cases:
        outputs = engine.evaluate_step(inputs)
        print(f"  Inputs: {inputs}, sum={sum(inputs)}", end="")
        if outputs:
            print(f" -> TRIGGERED! Output: {outputs[0]}")
        else:
            print(" -> No trigger")

def example_type_safety():
    """Demonstrate type safety benefits"""
    print("\n=== Type Safety Example ===")
    
    # This would have been error-prone with string names:
    # builder.add_add("sum", "inpt1", "inpt2")  # Typo!
    
    # With objects, typos are caught by Python:
    input1 = sdag.InputNode(0)
    input2 = sdag.InputNode(1)
    
    # IDE can autocomplete and type-check this:
    sum_node = sdag.AddNode(input1, input2)
    
    # Can't accidentally pass wrong type:
    # sum_node = sdag.AddNode(input1, "input2")  # Would raise error!
    
    print("Type safety prevents errors at DAG construction time!")
    print(f"Node IDs are automatically generated: {input1.node_id}, {sum_node.node_id}")

if __name__ == "__main__":
    example_object_composition()
    example_complex_dag()
    example_sum_node()
    example_type_safety()