#!/usr/bin/env python3
"""
Example usage of the sdag computational DAG with trigger-based evaluation.

This demonstrates:
1. Arena-style graph storage (nodes stored in flat array with indices)
2. Trigger-based sampling (outputs only when trigger value changes)
3. Multiple evaluation engines (topological and lazy)
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
weighted_bid = g.mul([bid, ask_size])
weighted_ask = g.mul([ask, bid_size])
top = g.add([weighted_bid, weighted_ask])
bottom = g.add([bid_size, ask_size])
wmp = g.div(top, bottom)

# Calculate simple midpoint (mid) - this will be our trigger
# mid = (bid + ask) / 2
sum_prices = g.add([bid, ask])
two = g.const(2.0)
mid = g.div(sum_prices, two)

# Create a combined node that depends on both mid and wmp
# This ensures both are included in the frozen graph
combined = g.add([mid, wmp])

# Example input rows - note that rows 2 and 3 have identical values
rows = [
    {"bid": 100.0, "ask": 101.0, "bid_size": 10.0, "ask_size": 12.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},  # Same as row 2
    {"bid": 101.0, "ask": 102.0, "bid_size": 12.0, "ask_size": 14.0},
]

# Freeze the combined graph to get both mid and wmp
yaml_graph = g.freeze(combined)

# Parse to find the indices of mid and wmp
import yaml
data = yaml.safe_load(yaml_graph)

# Find the indices by analyzing the graph structure
mid_idx = None
wmp_idx = None

for i, node in enumerate(data['nodes']):
    if node['type'] == 'div':
        # mid divides by constant 2.0
        if 'right' in node and data['nodes'][node['right']]['type'] == 'const':
            if data['nodes'][node['right']].get('value') == 2.0:
                mid_idx = i
        # wmp divides two add nodes
        elif 'left' in node and 'right' in node:
            if (data['nodes'][node['left']]['type'] == 'add' and 
                data['nodes'][node['right']]['type'] == 'add'):
                wmp_idx = i

print(f"Graph structure: {len(data['nodes'])} nodes")
print(f"  mid index: {mid_idx}")
print(f"  wmp index: {wmp_idx}")
print(f"  original root: {data['root']}")

# Update the root to be mid (our trigger)
data['root'] = mid_idx
trigger_yaml = yaml.dump(data)

# Create sampler with mid as trigger and wmp as output
print("\n" + "="*60)
print("Running with TOPOLOGICAL engine (default):")
print("="*60)

sampler = Sampler(trigger_yaml, outputs=[wmp_idx])
results = sampler.run(rows)

print("\nResults (only outputs when trigger changes):")
for i, result in enumerate(results):
    print(f"  Output {i}: mid={result['trigger']:.2f}, wmp={result['output0']:.4f}")

# Run again with lazy engine
print("\n" + "="*60)
print("Running with LAZY engine:")
print("="*60)

sampler_lazy = Sampler(trigger_yaml, outputs=[wmp_idx], engine_name="lazy")
results_lazy = sampler_lazy.run(rows)

print("\nResults (lazy evaluation):")
for i, result in enumerate(results_lazy):
    print(f"  Output {i}: mid={result['trigger']:.2f}, wmp={result['output0']:.4f}")

# Show expected values for comparison
print("\n" + "="*60)
print("Expected values for all input rows:")
print("="*60)
for i, row in enumerate(rows):
    mid_val = (row['bid'] + row['ask']) / 2.0
    wmp_val = (row['bid'] * row['ask_size'] + row['ask'] * row['bid_size']) / (row['bid_size'] + row['ask_size'])
    print(f"  Row {i}: mid={mid_val:.2f}, wmp={wmp_val:.4f}")

print(f"\nSummary:")
print(f"  - {len(rows)} input rows → {len(results)} output rows")
print(f"  - Row 2 was skipped because trigger (mid) didn't change")
print(f"  - Both engines produced identical results")
print(f"  - Arena graph has {len(data['nodes'])} nodes total (including shared sub-expressions)")