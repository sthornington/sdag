#!/usr/bin/env python3
"""
Working example of the sdag computational DAG with trigger-based evaluation.
The sampler reads input rows, evaluates the trigger DAG, and only outputs
when the trigger value changes.
"""
from sdag import *

# Build a computational graph
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

# To ensure both mid and wmp are in the graph, create a dummy node that uses both
# Then freeze with mid as the root (trigger)
dummy = g.add([mid, wmp])

# Freeze the graph with mid as the trigger (root)
trigger_yaml = g.freeze(mid)

# Parse to understand the graph structure
import yaml
graph_data = yaml.safe_load(trigger_yaml)
print("Graph structure:")
print(f"Total nodes: {len(graph_data['nodes'])}")
print(f"Root (trigger) index: {graph_data['root']}")

# Since mid is the root, we need to find wmp's index
# We'll look for it systematically
wmp_candidates = []
for i, node in enumerate(graph_data['nodes']):
    if node['type'] == 'div' and i != graph_data['root']:
        print(f"Found div node at index {i}: {node}")
        wmp_candidates.append(i)

# For this example, we know wmp should be one of the div nodes
# Let's just use the first div node that's not the root
wmp_index = wmp_candidates[0] if wmp_candidates else None

if wmp_index is None:
    # If wmp is not in the mid graph, we need to freeze from a different root
    # Freeze from wmp instead and use mid as the trigger
    print("\nWMP not found in mid's graph, freezing from dummy node instead...")
    full_yaml = g.freeze(dummy)
    graph_data = yaml.safe_load(full_yaml)
    
    # Find both mid and wmp indices in the full graph
    mid_index = None
    wmp_index = None
    for i, node in enumerate(graph_data['nodes']):
        if node['type'] == 'div':
            # The div node with a const 2.0 as right operand is mid
            if 'right' in node and graph_data['nodes'][node['right']]['type'] == 'const':
                if graph_data['nodes'][node['right']].get('value') == 2.0:
                    mid_index = i
            # The other div nodes could be wmp
            elif mid_index != i:
                wmp_index = i
    
    print(f"In full graph: mid={mid_index}, wmp={wmp_index}")
    
    # Use the full graph with mid as root
    # Note: the sampler expects root to be the trigger
    # So we'll modify the graph data to set mid as root
    graph_data['root'] = mid_index
    trigger_yaml = yaml.dump(graph_data)

print("\n" + "="*60 + "\n")

# Example rows - note that rows 2 and 3 have identical values
rows = [
    {"bid": 100.0, "ask": 101.0, "bid_size": 10.0, "ask_size": 12.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},  # Same as row 2
    {"bid": 101.0, "ask": 102.0, "bid_size": 12.0, "ask_size": 14.0},
]

# Create sampler with mid as trigger and wmp as output
print(f"Creating sampler with trigger at root and output at index {wmp_index}")
sampler = Sampler(trigger_yaml, outputs=[wmp_index])

# Run the sampler
results = sampler.run(rows)

print("\nSampler Results (only outputs when trigger changes):")
print("-" * 50)
for i, result in enumerate(results):
    print(f"Output row {i}: {result}")

print("\n" + "="*60 + "\n")
print("Analysis:")
print(f"- Input had {len(rows)} rows")
print(f"- Output has {len(results)} rows (row 2 and 3 had same trigger value)")
print(f"- Each output row contains: trigger (mid value) and output0 (wmp value)")

# Verify the calculations manually
print("\nManual verification of expected values:")
for i, row in enumerate(rows):
    mid_calc = (row['bid'] + row['ask']) / 2.0
    wmp_calc = (row['bid'] * row['ask_size'] + row['ask'] * row['bid_size']) / (row['bid_size'] + row['ask_size'])
    print(f"Row {i}: mid={mid_calc:.2f}, wmp={wmp_calc:.4f}")