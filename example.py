#!/usr/bin/env python3
"""
Example usage of the sdag computational DAG DSL and evaluation.
"""
from sdag import *

# Build a simple weighted midpoint vs. weighted mean-price example
bid = InputNode("bid")
bid_size = InputNode("bid_size")
ask = InputNode("ask")
ask_size = InputNode("ask_size")
top = Add([Mul([bid, ask_size]), Mul([ask, bid_size])])
bottom = Add([bid_size, ask_size])
wmp = Div(top, bottom)
# initialize factory before using g
g = Graph()
mid = Mul([g.add([bid, ask]), g.const(0.5)])

# Example rows as list of Python dicts
rows = [
    {"bid": 100.0, "ask": 101.0, "bid_size": 10.0, "ask_size": 12.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},
    {"bid": 101.0, "ask": 102.0, "bid_size": 12.0, "ask_size": 14.0},
]

# Create and run sampler entirely in Rust
mid_yaml = freeze(mid)
wmp_yaml = freeze(wmp)

s = Sampler(trigger=mid_yaml, output=[mid_yaml, wmp_yaml])
results = s.run(rows)

print("Trigger changed values with outputs:")
for r in results:
    print(r)
