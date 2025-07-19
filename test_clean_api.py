#!/usr/bin/env python3
"""
Test the cleaner Python API design:
- Input and Constant are special (leaf nodes)
- All transform nodes use a generic macro
- Adding a new transform node requires just one line: transform_node!(NewNode);
"""

import sdag

def test_clean_api():
    """Test the cleaner separation of concerns"""
    print("=== Testing Clean API ===")
    
    # Create a graph
    graph = sdag.Graph()
    
    # Leaf nodes have custom constructors
    a = sdag.Input(graph, 0)  # input_index
    b = sdag.Input(graph, 1)
    threshold = sdag.Constant(graph, 100.0)  # value
    
    # Transform nodes all work the same way:
    # - Positional args are node inputs
    # - Kwargs are additional parameters
    total = sdag.Add(graph, a, b)
    alert = sdag.Comparison(graph, total, threshold, op="GreaterThan")
    
    # Mark outputs and trigger
    total.output()
    alert.trigger()
    
    # Build engine
    engine = graph.build_engine()
    
    # Test it
    print("\nTesting with values:")
    test_cases = [
        (45, 50),   # 95 < 100
        (55, 55),   # 110 > 100  
        (40, 45),   # 85 < 100
    ]
    
    for a_val, b_val in test_cases:
        outputs = engine.evaluate_step([a_val, b_val])
        print(f"  {a_val} + {b_val} = {a_val + b_val}", end="")
        if outputs:
            print(f" -> TRIGGERED! Output = {outputs[0]}")
        else:
            print(" -> No trigger")

def test_transform_nodes():
    """Show how all transform nodes work the same way"""
    print("\n=== Testing Transform Nodes ===")
    
    graph = sdag.Graph()
    
    # Inputs
    x = sdag.Input(graph, 0)
    y = sdag.Input(graph, 1)
    z = sdag.Input(graph, 2)
    
    # All transform nodes follow the same pattern
    # Node inputs as positional args:
    sum_xy = sdag.Add(graph, x, y)
    
    # Additional parameters as kwargs:
    doubled = sdag.ConstantProduct(graph, sum_xy, factor=2.0)
    tripled = sdag.ConstantProduct(graph, sum_xy, factor=3.0)
    
    # Comparison with op parameter:
    threshold = sdag.Constant(graph, 50.0)
    is_high = sdag.Comparison(graph, doubled, threshold, op="GreaterThan")
    
    # Sum can take a list of nodes
    total = sdag.Sum(graph, [x, y, z])
    
    # Always trigger
    sdag.Constant(graph, 1.0).trigger()
    
    # Outputs
    doubled.output()
    tripled.output()
    total.output()
    
    print("\nGenerated YAML:")
    print(graph.to_yaml())
    
    # Test
    engine = graph.build_engine()
    outputs = engine.evaluate_step([10, 20, 30])
    if outputs:
        print(f"\nInputs: [10, 20, 30]")
        print(f"x + y = 30")
        print(f"Doubled: {outputs[0]}")
        print(f"Tripled: {outputs[1]}") 
        print(f"Total: {outputs[2]}")

def show_how_to_add_new_node():
    """Demonstrate adding a new node type"""
    print("\n=== How to Add a New Node Type ===")
    print("""
To add a new transform node (e.g., 'Subtract'):

1. Add to NodeOp enum in nodes.rs:
   Subtract { a: usize, b: usize }

2. Add compute logic in engine.rs:
   NodeOp::Subtract { a, b } => {
       self.values[*a] - self.values[*b]
   }

3. Add one line to python.rs:
   transform_node!(Subtract);

That's it! The Python API is automatically generated.
No special cases, no parameter mapping logic.

For leaf nodes (like Input/Constant), you need custom code
because they have unique constructors.
""")

if __name__ == "__main__":
    test_clean_api()
    test_transform_nodes()
    show_how_to_add_new_node()
    print("\n✅ All tests completed!")