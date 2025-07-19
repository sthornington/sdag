# Simple Node System for SDAG

This document describes the simplified node system that was implemented to make it dead simple to add new nodes.

## Overview

The system provides:
1. Arena-style graph storage with flat arrays and node IDs
2. Multiple evaluation engines (topological and lazy)
3. Trigger-based evaluation (only outputs when trigger changes)
4. Simple manual node definitions (no complex macros)

## How to Add a New Node

Adding a new node requires just a few steps:

### 1. Define the Node Struct

```rust
#[derive(Debug, Clone)]
pub struct MyNode {
    pub field1: f64,
    pub field2: NodeId,
    pub field3: Vec<NodeId>,
}
```

### 2. Implement the EvalNode Trait

```rust
impl EvalNode for MyNode {
    fn eval(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        // Your computation logic here
        values[self.field2] + self.field1
    }
}

impl ArenaEval for MyNode {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.eval(values, inputs)
    }
}
```

### 3. Create Python Wrapper

```rust
#[pyclass]
pub struct My {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub field1: f64,
    #[pyo3(get)]
    pub field2: PyObject,
    #[pyo3(get)]
    pub field3: Vec<PyObject>,
}
```

### 4. Add Graph Builder Method

```rust
impl Graph {
    fn my(&mut self, py: Python, field1: f64, field2: PyObject, field3: Vec<PyObject>) -> PyObject {
        let id = format!("n{}", self.counter);
        self.counter += 1;
        let node = My { id: id.clone(), field1, field2, field3 };
        let py_node = node.into_py(py);
        self.registry.insert(id, py_node.clone());
        py_node
    }
}
```

### 5. Handle in Sampler

Add to the match statement in `Sampler::run()`:

```rust
"my" => {
    let field1 = match arena_node.fields.get("field1") {
        Some(engine::FieldValue::Float(f)) => *f,
        _ => return Err(pyo3::exceptions::PyValueError::new_err("my node missing field1")),
    };
    let field2 = match arena_node.fields.get("field2") {
        Some(engine::FieldValue::One(id)) => *id,
        _ => return Err(pyo3::exceptions::PyValueError::new_err("my node missing field2")),
    };
    let field3 = match arena_node.fields.get("field3") {
        Some(engine::FieldValue::Many(ids)) => ids.clone(),
        _ => return Err(pyo3::exceptions::PyValueError::new_err("my node missing field3")),
    };
    Box::new(MyNode { field1, field2, field3 })
},
```

### 6. Handle in freeze_graph

Add to the node type matching and field extraction.

### 7. Register with Python Module

```rust
m.add_class::<My>()?;
```

## Benefits of This Approach

1. **Dead Simple**: No complex macros or code generation
2. **Type Safe**: Full Rust type safety
3. **Efficient**: Arena allocation with flat arrays
4. **Flexible**: Easy to add custom logic
5. **Maintainable**: All code is explicit and visible

## Architecture

- **Arena Storage**: Nodes are stored in a flat Vec with NodeId indices
- **Shared Nodes**: Identical nodes are automatically deduplicated
- **Lazy Evaluation**: Only computes what's needed
- **Trigger-Based Output**: Only outputs when trigger value changes

## Example Usage

```python
import sdag

# Build a graph
g = sdag.Graph()
a = g.input("a")
b = g.input("b")
sum_ab = g.add([a, b])
result = g.mul([sum_ab, g.const(2.0)])

# Freeze to YAML
yaml_str = g.freeze(result)

# Create sampler
sampler = sdag.Sampler(yaml_str, outputs=[1, 2], engine_name="lazy")

# Run with inputs
results = sampler.run([
    {"a": 1.0, "b": 2.0},
    {"a": 1.0, "b": 3.0},
])
```

## Next Steps

To make this even simpler, you could:
1. Use a registration system to auto-generate the Sampler match arms
2. Add a derive macro for the Python wrapper boilerplate
3. Create a plugin system for dynamically loading nodes

But the current system achieves the goal of being dead simple while maintaining all the required functionality.