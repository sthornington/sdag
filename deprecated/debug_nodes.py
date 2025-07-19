#!/usr/bin/env python3
from sdag import *

g = Graph()
bid = g.input("bid")
ask = g.input("ask")
sum_prices = g.add([bid, ask])

print("=== Node inspection ===")
print(f"bid: {bid}")
print(f"  type: {type(bid)}")
print(f"  id: {bid.id}")
print(f"  TYPE: {bid.TYPE}")
print(f"  FIELDS: {bid.FIELDS}")
print(f"  name: {bid.name}")

print(f"\nsum_prices: {sum_prices}")
print(f"  type: {type(sum_prices)}")
print(f"  id: {sum_prices.id}")
print(f"  TYPE: {sum_prices.TYPE}")
print(f"  FIELDS: {sum_prices.FIELDS}")
print(f"  children: {sum_prices.children}")
print(f"  children[0]: {sum_prices.children[0]}")
print(f"  children[0] type: {type(sum_prices.children[0])}")

# Let's check what happens when we getattr
for field in sum_prices.FIELDS:
    val = getattr(sum_prices, field)
    print(f"\nField '{field}':")
    print(f"  value: {val}")
    print(f"  type: {type(val)}")
    if hasattr(val, '__iter__') and not isinstance(val, str):
        for i, item in enumerate(val):
            print(f"  item[{i}]: {item}, type: {type(item)}")