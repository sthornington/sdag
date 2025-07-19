#!/usr/bin/env python3
from sdag import *

g = Graph()
x = g.input("test_name")

print(f"x type: {type(x)}")
print(f"x.id: {x.id}")
print(f"x.name: {x.name}")
print(f"x.TYPE: {x.TYPE}")
print(f"x.FIELDS: {x.FIELDS}")

# Check what getattr returns for each field
import sys
for field in x.FIELDS:
    val = getattr(x, field)
    print(f"Field '{field}': type={type(val)}, value={val}")