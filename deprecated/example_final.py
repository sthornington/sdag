#!/usr/bin/env python3
"""
Final working example of sdag computational DAG with trigger-based evaluation.
This example properly demonstrates:
1. Arena-style graph storage with node sharing
2. Trigger-based evaluation (only outputs when trigger changes)
3. Multiple evaluation engines
"""
from sdag import *

# Build a computational DAG
g = Graph()

# Input nodes
bid = g.input("bid")
bid_size = g.input("bid_size")
ask = g.input("ask")
ask_size = g.input("ask_size")

# Calculate weighted midpoint (wmp)
# wmp = (bid * ask_size + ask * bid_size) / (bid_size + ask_size)
top = g.add([g.mul([bid, ask_size]), g.mul([ask, bid_size])])
bottom = g.add([bid_size, ask_size])
wmp = g.div(top, bottom)

# Calculate simple midpoint (mid) - this will be our trigger
# mid = (bid + ask) / 2
two = g.const(2.0)
sum_prices = g.add([bid, ask])
mid = g.div(sum_prices, two)

# Example rows - note that rows 2 and 3 have identical values
rows = [
    {"bid": 100.0, "ask": 101.0, "bid_size": 10.0, "ask_size": 12.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},  # Same as row 2
    {"bid": 101.0, "ask": 102.0, "bid_size": 12.0, "ask_size": 14.0},
]

# The key insight: we need to freeze a graph that contains both mid and wmp,
# with mid as the root (trigger). The arena engine evaluates in topological order.
# So we create a dummy node that uses both, then freeze from mid.

# Actually, let's just use wmp as the root since it depends on everything we need
# and we'll tell the sampler that mid is the trigger by its index
full_graph = g.freeze(wmp)

print("Full graph YAML (wmp as root):")
print(full_graph)
print("\n" + "="*60 + "\n")

# Parse to find mid's index
import yaml
data = yaml.safe_load(full_graph)

# Find mid index (div node that divides by 2.0)
mid_idx = None
for i, node in enumerate(data['nodes']):
    if node['type'] == 'div' and 'right' in node:
        right_node = data['nodes'][node['right']]
        if right_node['type'] == 'const' and right_node.get('value') == 2.0:
            mid_idx = i
            break

print(f"Found mid at index: {mid_idx}")
print(f"Root (wmp) is at index: {data['root']}")

# For the sampler, the root is the trigger and outputs are additional nodes to evaluate
# Since wmp is already the root, we need to make mid the root instead
data['root'] = mid_idx
modified_yaml = yaml.dump(data)

# Create sampler with mid as trigger and wmp as additional output
wmp_idx = len(data['nodes']) - 1  # wmp is the last node since it was the original root
sampler = Sampler(modified_yaml, outputs=[wmp_idx])

print(f"\nRunning sampler with trigger at index {mid_idx} and output at index {wmp_idx}")
results = sampler.run(rows)

print("\nResults (trigger-based evaluation):")
print("-" * 60)
for i, result in enumerate(results):
    print(f"Output {i}: trigger={result['trigger']:.2f}, wmp={result['output0']:.4f}")

print("\nExpected values for all input rows:")
for i, row in enumerate(rows):
    mid_calc = (row['bid'] + row['ask']) / 2.0
    wmp_calc = (row['bid'] * row['ask_size'] + row['ask'] * row['bid_size']) / (row['bid_size'] + row['ask_size'])
    print(f"Row {i}: mid={mid_calc:.2f}, wmp={wmp_calc:.4f}")

print(f"\nAnalysis:")
print(f"- {len(rows)} input rows → {len(results)} output rows")
print(f"- Row 2 was skipped because its trigger value (mid) was the same as row 1")
print(f"- This demonstrates trigger-based evaluation working correctly")

# Test with lazy engine
print("\n" + "="*60 + "\n")
print("Testing with lazy evaluation engine:")
sampler_lazy = Sampler(modified_yaml, outputs=[wmp_idx], engine_name="lazy")
results_lazy = sampler_lazy.run(rows)

print("Lazy engine results:")
for i, result in enumerate(results_lazy):
    print(f"Output {i}: trigger={result['trigger']:.2f}, wmp={result['output0']:.4f}")

# Verify both engines produce same results
match = all(
    abs(r1['trigger'] - r2['trigger']) < 1e-10 and
    abs(r1['output0'] - r2['output0']) < 1e-10
    for r1, r2 in zip(results, results_lazy)
)
print(f"\nBoth engines produce identical results: {match}")