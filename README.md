# SDAG - Simple DAG Execution Engine

A Rust-based DAG (Directed Acyclic Graph) execution engine with Python bindings.

## Features

- Define computation nodes in Rust with minimal boilerplate
- YAML serialization for DAG definitions
- Python bindings for easy scripting
- Extensible node system
- Pluggable evaluation strategies (future enhancement)

## Building

### Rust Library
```bash
cargo build --release
cargo test
```

### Python Extension
```bash
pip install maturin
maturin develop
```

## Usage

### From Python
```python
import sdag

# Create a DAG
dag = sdag.PyDag()

# Add nodes
dag.add_node("x", "Constant", {"value": 10.0})
dag.add_node("y", "Constant", {"value": 20.0})
dag.add_node("add", "Add")

# Connect nodes
dag.connect("x", "value", "add", "a")
dag.connect("y", "value", "add", "b")

# Execute
results = dag.execute()
print(results["add"]["result"])  # 30.0

# Save to YAML
dag.save_yaml("my_dag.yaml")
```

### From Rust (YAML)
```bash
cargo run -- my_dag.yaml
```

## Extending with New Nodes

To add a new node type, implement the `Node` trait:

```rust
struct MyNode {
    parameter: f64,
}

impl Node for MyNode {
    fn compute(&self, inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>> {
        // Your computation logic here
    }
}
```

Then register it in the `NodeRegistry`.