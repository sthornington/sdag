# Design Issue with Python API

The user correctly identified that my macro-based approach is not truly generic. I have special handling for different parameter types:
- Input wants `input_index`
- Constant wants `value` 
- ConstantProduct wants `factor`
- Comparison wants `op`

This violates the principle of making it easy to add new nodes.

## The Core Problem

The YAML format has each node type define its own parameter names:
```yaml
- type: Input
  params:
    input_index: 0
    
- type: Constant
  params:
    value: 5.0
    
- type: ConstantProduct
  params:
    inputs: [node1]
    factor: 2.0
```

But the user wants a Python API like:
```python
a = sdag.Input(graph, 0)
b = sdag.Constant(graph, 5.0)
c = sdag.ConstantProduct(graph, a, factor=2.0)
```

To make this work generically, I need conventions to map positional args to parameter names. But any convention I choose (first int -> input_index, first float -> value) is inherently node-specific.

## Possible Solutions

1. **Accept the YAML structure in Python**: Make users specify parameter names explicitly
   ```python
   a = sdag.Input(graph, input_index=0)
   b = sdag.Constant(graph, value=5.0)
   ```

2. **Use a builder pattern**: But the user explicitly rejected this

3. **Generate Python bindings from the node definitions**: Use a build script that reads the NodeOp enum and generates appropriate Python classes

4. **Accept some boilerplate**: Each node type needs a small Python wrapper that defines its parameter mapping

The user wants option 3 - automatic generation from the enum definition.