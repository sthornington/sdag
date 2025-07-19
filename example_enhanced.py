#!/usr/bin/env python3
"""
Enhanced example demonstrating the refactored sdag computational DAG with:
1. Arena-style graph storage with shared nodes
2. Multiple evaluation engines (topological and lazy)
3. Comprehensive macro-based node definitions
"""
from sdag import *

# Build a computational graph with shared nodes
g = Graph()

# Input nodes
bid = g.input("bid")
bid_size = g.input("bid_size")
ask = g.input("ask")
ask_size = g.input("ask_size")

# Shared intermediate calculations
weighted_bid = g.mul([bid, ask_size])  # Shared node
weighted_ask = g.mul([ask, bid_size])  # Shared node

# Calculate weighted midpoint (uses shared nodes)
top = g.add([weighted_bid, weighted_ask])
bottom = g.add([bid_size, ask_size])
wmp = g.div(top, bottom)

# Calculate simple midpoint
two = g.const(2.0)
sum_price = g.add([bid, ask])
mid = g.div(sum_price, two)

# Calculate spread using shared nodes
spread = g.add([weighted_ask, weighted_bid])  # Reuses shared nodes

# Example input data
rows = [
    {"bid": 100.0, "ask": 101.0, "bid_size": 10.0, "ask_size": 12.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},
    {"bid": 101.0, "ask": 102.0, "bid_size": 12.0, "ask_size": 14.0},
    {"bid": 101.5, "ask": 102.5, "bid_size": 15.0, "ask_size": 10.0},
]

# Freeze the graph into arena format (nodes are deduplicated)
graph_yaml = g.freeze(wmp)
print("Arena-style graph YAML:")
print(graph_yaml)
print("\n" + "="*60 + "\n")

# Parse YAML to get node indices
import yaml
graph_data = yaml.safe_load(graph_yaml)

# Find the node indices for our outputs
node_indices = {}
for node in graph_data['nodes']:
    node_id = node['id']
    # We'll need to match against the original graph to find which nodes we want
    node_indices[node_id] = node

# For this example, let's output the root node and a few others
# The root is wmp, we can also output some intermediate nodes
output_indices = [graph_data['root']]  # Start with root (wmp)

# Test with topological engine (default)
print("Topological Engine Results:")
print("-" * 30)
sampler_topo = Sampler(graph_yaml, outputs=output_indices)
results_topo = sampler_topo.run(rows)
for i, result in enumerate(results_topo):
    print(f"Row {i}: {result}")

print("\n" + "="*60 + "\n")

# Test with lazy evaluation engine
print("Lazy Engine Results:")
print("-" * 30)
sampler_lazy = Sampler(graph_yaml, outputs=output_indices, engine_name="lazy")
results_lazy = sampler_lazy.run(rows)
for i, result in enumerate(results_lazy):
    print(f"Row {i}: {result}")

print("\n" + "="*60 + "\n")

# Verify both engines produce same results
print("Verification:")
all_match = all(
    all(abs(r1[k] - r2[k]) < 1e-10 for k in r1.keys())
    for r1, r2 in zip(results_topo, results_lazy)
)
print(f"Both engines produce identical results: {all_match}")

# Demonstrate node sharing in the arena
print("\n" + "="*60 + "\n")
print("Node Sharing Analysis:")
print("-" * 30)

# Parse the YAML to analyze node sharing
import yaml
graph_data = yaml.safe_load(graph_yaml)
node_types = {}
for node in graph_data['nodes']:
    node_type = node['type']
    node_types[node_type] = node_types.get(node_type, 0) + 1

print(f"Total nodes in arena: {len(graph_data['nodes'])}")
print("Node type distribution:")
for node_type, count in sorted(node_types.items()):
    print(f"  {node_type}: {count}")

# Show that weighted_bid and weighted_ask are stored only once each
print(f"\nNote: Despite multiple references, shared nodes like 'weighted_bid' and 'weighted_ask'")
print(f"are stored only once in the arena, demonstrating efficient memory usage.")