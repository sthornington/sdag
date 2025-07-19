# How to Add a New Node Type

To add a new node type (e.g., Pow for exponentiation), you need to make changes in **TWO** places:

## 1. Add to NodeOp enum in `src/nodes.rs`:
```rust
/// Power: a^b
Pow { inputs: Vec<usize> },
```

## 2. Add compute logic in `src/engine.rs` compute_node():
```rust
NodeOp::Pow { inputs } => {
    let base = *self.values.get_unchecked(inputs[0]);
    let exp = *self.values.get_unchecked(inputs[1]);
    base.powf(exp)
}
```

## For Python support:
Add one line in `src/python.rs`:
```rust
transform_node!(Pow);
```

## That's it!

The YAML format now uses direct indices, so there's no string resolution needed.
Example YAML:
```yaml
nodes:
  - !Input { input_index: 0 }
  - !Constant { value: 2.0 }
  - !Pow { inputs: [0, 1] }  # 0 references first node, 1 references second
trigger: 2
outputs: [2]
```