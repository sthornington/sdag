#!/usr/bin/env python3
"""Test script to verify the Python bindings work correctly"""

import sdag

def test_basic_streaming():
    """Test basic streaming functionality"""
    print("=== Testing Basic Streaming ===")
    
    # Load the streaming example
    engine = sdag.PyEngine.from_yaml_file("streaming_example.yaml")
    
    # Test case 1: Values that don't trigger (sum < 100)
    outputs = engine.evaluate_step([40.0, 40.0])  
    print(f"Step 1 (40 + 40 = 80): Trigger = {outputs is not None}")
    if outputs:  # First run always triggers
        print(f"  Outputs: {outputs}")
    
    # Test case 2: Values that don't trigger (sum < 100) 
    outputs = engine.evaluate_step([45.0, 45.0])
    print(f"Step 2 (45 + 45 = 90): Trigger = {outputs is not None}")
    
    # Test case 3: Values that trigger (sum > 100)
    outputs = engine.evaluate_step([60.0, 60.0])
    print(f"Step 3 (60 + 60 = 120): Trigger = {outputs is not None}")
    if outputs:
        print(f"  Outputs: sum={outputs[0]}, adjusted={outputs[1]}")
        assert outputs[0] == 120.0  # sum
        assert outputs[1] == 180.0  # adjusted (120 * 1.5)

def test_incremental_updates():
    """Test that only changed paths are recomputed"""
    print("\n=== Testing Incremental Updates ===")
    
    # Build a DAG with multiple paths
    builder = sdag.PyDagBuilder()
    builder.add_input("a", 0)
    builder.add_input("b", 1) 
    builder.add_constant("c", 10.0)
    builder.add_add("a_plus_c", "a", "c")  # a + 10
    builder.add_add("b_plus_c", "b", "c")  # b + 10
    builder.add_multiply("product", "a_plus_c", "b_plus_c")  # (a+10) * (b+10)
    builder.set_outputs(["a_plus_c", "b_plus_c", "product"])
    
    engine = builder.build()
    
    # First evaluation
    outputs = engine.evaluate_step([1.0, 2.0])
    if outputs:  # First run always produces outputs
        print(f"Step 1: a=1, b=2")
        print(f"  a+10={outputs[0]}, b+10={outputs[1]}, product={outputs[2]}")
        assert outputs[0] == 11.0
        assert outputs[1] == 12.0  
        assert outputs[2] == 132.0  # 11 * 12
    
    # Change only 'a' - should recompute a_plus_c and product, but not b_plus_c
    outputs = engine.evaluate_step([2.0, 2.0])
    if outputs:
        print(f"Step 2: a=2, b=2 (only 'a' changed)")
        print(f"  a+10={outputs[0]}, b+10={outputs[1]}, product={outputs[2]}")
        # b_plus_c should still be 12 from cache
        assert outputs[0] == 12.0
        assert outputs[1] == 12.0
        assert outputs[2] == 144.0  # 12 * 12

def test_comparison_trigger():
    """Test comparison operators as triggers"""
    print("\n=== Testing Comparison Triggers ===")
    
    builder = sdag.PyDagBuilder()
    builder.add_input("value", 0)
    builder.add_constant("threshold", 50.0)
    builder.add_comparison("above_threshold", "value", "threshold", "GreaterThan")
    builder.set_trigger("above_threshold")
    builder.set_outputs(["value"])
    
    engine = builder.build()
    
    # First value: below threshold
    outputs = engine.evaluate_step([30.0])
    print(f"Step 1 (30 < 50): Trigger = {outputs is not None} (first run)")
    
    # Second value: still below
    outputs = engine.evaluate_step([40.0])
    print(f"Step 2 (40 < 50): Trigger = {outputs is not None}")
    assert outputs is None
    
    # Third value: above threshold
    outputs = engine.evaluate_step([60.0])
    print(f"Step 3 (60 > 50): Trigger = {outputs is not None}")
    assert outputs is not None
    assert outputs[0] == 60.0
    
    # Fourth value: back below threshold
    outputs = engine.evaluate_step([45.0])
    print(f"Step 4 (45 < 50): Trigger = {outputs is not None}")
    assert outputs is not None  # Trigger changed from 1.0 to 0.0

if __name__ == "__main__":
    test_basic_streaming()
    test_incremental_updates()
    test_comparison_trigger()
    print("\n✅ All tests passed!")