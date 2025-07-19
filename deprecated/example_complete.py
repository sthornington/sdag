#!/usr/bin/env python3
"""
Complete example of the sdag computational DAG with trigger-based evaluation.
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

# Calculate simple midpoint (mid)
# mid = (bid + ask) / 2
sum_prices = g.add([bid, ask])
two = g.const(2.0)
mid = g.div(sum_prices, two)

# Example rows - note that rows 2 and 3 have identical values
rows = [
    {"bid": 100.0, "ask": 101.0, "bid_size": 10.0, "ask_size": 12.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},  # Same as row 2
    {"bid": 101.0, "ask": 102.0, "bid_size": 12.0, "ask_size": 14.0},
]

# First, create a combined node that references both mid and wmp
# This ensures both are included in the frozen graph
both = g.add([mid, wmp])

# Freeze the combined graph
full_yaml = g.freeze(both)
print("Full graph YAML (includes both mid and wmp):")
print(full_yaml)
print("\n" + "="*60 + "\n")

# Parse to find the node indices
import yaml
graph_data = yaml.safe_load(full_yaml)

# Find mid and wmp indices
mid_index = None
wmp_index = None

# We know the structure: mid = sum_prices / 2.0, wmp = top / bottom
# So we can identify them by their operands
for i, node in enumerate(graph_data['nodes']):
    if node['type'] == 'div':
        # Check if this is mid (divides by constant 2.0)
        if 'right' in node:
            right_node = graph_data['nodes'][node['right']]
            if right_node['type'] == 'const' and right_node.get('value') == 2.0:
                mid_index = i
        # Check if this is wmp (divides top by bottom, both are add nodes)
        if 'left' in node and 'right' in node:
            left_node = graph_data['nodes'][node['left']]
            right_node = graph_data['nodes'][node['right']]
            if left_node['type'] == 'add' and right_node['type'] == 'add':
                # This is likely wmp (top/bottom)
                wmp_index = i

print(f"Mid index: {mid_index}")
print(f"WMP index: {wmp_index}")

# Now we need to re-freeze with just mid as the trigger
# but keeping the wmp node accessible
trigger_yaml = g.freeze(mid)
print(f"\nTrigger graph (mid as root): root index = {yaml.safe_load(trigger_yaml)['root']}")
print("\n" + "="*60 + "\n")

# Create sampler with mid as trigger and wmp as output
sampler = Sampler(trigger_yaml, outputs=[wmp_index])

# Run the sampler
results = sampler.run(rows)

print("Sampler Results (only outputs when trigger changes):")
print("-" * 50)
for i, result in enumerate(results):
    print(f"Output {i}: {result}")

print("\n" + "="*60 + "\n")
print("Analysis:")
print(f"- Input had {len(rows)} rows")
print(f"- Output has {len(results)} rows (because row 2 and 3 had same trigger value)")
print(f"- Each output row contains: trigger (mid value) and output0 (wmp value)")

# Verify the calculations
print("\nManual verification of results:")
for i, row in enumerate(rows):
    mid_calc = (row['bid'] + row['ask']) / 2.0
    wmp_calc = (row['bid'] * row['ask_size'] + row['ask'] * row['bid_size']) / (row['bid_size'] + row['ask_size'])
    print(f"Row {i}: mid={mid_calc:.2f}, wmp={wmp_calc:.4f}")