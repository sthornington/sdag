#!/usr/bin/env python3
from sdag import *

g = Graph()
x = g.input("x")
y = g.input("y")
z = g.add([x, y])

print(f"x type: {type(x)}, x.id: {x.id}")
print(f"y type: {type(y)}, y.id: {y.id}")
print(f"z type: {type(z)}, z.id: {z.id}")

try:
    yaml = g.freeze(z)
    print(f"Success! YAML:\n{yaml}")
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()