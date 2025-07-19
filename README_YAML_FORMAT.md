# YAML Format

The DAG uses a simplified YAML format with direct integer indices (no string node references).

## Structure

```yaml
nodes:
  - type: NodeType
    params:
      field1: value1
      field2: value2
trigger: <node_index>  # Optional: which node triggers output
outputs: [<indices>]   # Optional: which nodes to output when triggered
```

## Node Types

### Input
```yaml
- type: Input
  params:
    input_index: 0  # Which streaming input slot (0-based)
```

### Constant
```yaml
- type: Constant
  params:
    value: 42.0
```

### Add
```yaml
- type: Add
  params:
    inputs: [0, 1]  # Indices of nodes to add
```

### Multiply
```yaml
- type: Multiply
  params:
    inputs: [0, 1]  # Indices of nodes to multiply
```

### Sum
```yaml
- type: Sum
  params:
    inputs: [0, 1, 2, 3]  # Indices of nodes to sum
```

### ConstantProduct
```yaml
- type: ConstantProduct
  params:
    inputs: [0]     # Input node index
    factor: 2.5     # Multiply by this constant
```

### Comparison
```yaml
- type: Comparison
  params:
    inputs: [0, 1]  # [a, b]
    op: GreaterThan # GreaterThan | LessThan | Equal
```

## Example

```yaml
nodes:
  - type: Input          # 0: price_a
    params:
      input_index: 0
  - type: Input          # 1: price_b
    params:
      input_index: 1
  - type: Constant       # 2: threshold
    params:
      value: 100.0
  - type: Add            # 3: sum = price_a + price_b
    params:
      inputs: [0, 1]
  - type: Comparison     # 4: check = sum > threshold
    params:
      inputs: [3, 2]
      op: GreaterThan
trigger: 4               # Fire when check is true
outputs: [3]             # Output the sum
```

## Important Notes

- Nodes are referenced by their index in the nodes array (0-based)
- No string IDs or names - just integer indices
- Nodes must be in topological order (dependencies before dependents)
- The format is not meant to be human-editable for large graphs