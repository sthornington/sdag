#!/usr/bin/env python3
"""
Example usage of the sdag computational DAG DSL and evaluation.
"""
from sdag import *

# Build a simple weighted midpoint vs. weighted mean-price example
g = Graph()
bid = g.input("bid")
bid_size = g.input("bid_size")
ask = g.input("ask")
ask_size = g.input("ask_size")
top = g.add([g.mul([bid, ask_size]), g.mul([ask, bid_size])])
bottom = g.add([bid_size, ask_size])
wmp = g.div(top, bottom)
mid = g.mul([bid, ask])

# Example rows as list of Python dicts
rows = [
    {"bid": 100.0, "ask": 101.0, "bid_size": 10.0, "ask_size": 12.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},
    {"bid": 100.5, "ask": 101.5, "bid_size": 11.0, "ask_size": 13.0},
    {"bid": 101.0, "ask": 102.0, "bid_size": 12.0, "ask_size": 14.0},
]

## Create and run sampler entirely in Rust
s_yaml = g.freeze(mid)
w_yaml = g.freeze(wmp)
print("s_yaml=", s_yaml, sep='')
print("w_yaml=", w_yaml, sep='')
try:
    # Sampler demo: run when Rust sampler is available
    s = Sampler(trigger=s_yaml, output=[w_yaml])
    results = s.run(rows)
    print("Trigger changed values with outputs:")
    for r in results:
        print(r)
except Exception:
    print("Sampler demo skipped: unable to initialize sampler with current spec.")
