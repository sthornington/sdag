#!/usr/bin/env python3
"""Test simple graph creation and freezing"""
from sdag import *

g = Graph()

# Create a very simple graph
x = g.const(2.0)
y = g.const(3.0)
z = g.add([x, y])

print(f"x: {x}, type: {type(x)}")
print(f"y: {y}, type: {type(y)}")
print(f"z: {z}, type: {type(z)}")
print(f"z.children: {z.children}")

# Try to freeze
try:
    yaml = g.freeze(z)
    print(f"\nSuccess! YAML:\n{yaml}")
except Exception as e:
    print(f"\nError during freeze: {e}")
    import traceback
    traceback.print_exc()