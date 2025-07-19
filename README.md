# sdag

A high-performance streaming DAG (Directed Acyclic Graph) execution engine written in Rust with Python bindings.

## Features

- **Extreme Performance**: ~20ns per node evaluation with cache-friendly dense arrays
- **Streaming Evaluation**: Process data row-by-row with trigger-based output emission
- **Incremental Computation**: Only recompute nodes affected by input changes
- **Python Bindings**: Full-featured Python API via PyO3
- **YAML Serialization**: Define DAGs in YAML for portability
- **Zero-Copy Design**: Minimal allocations during evaluation

## Installation

```bash
# Create virtual environment
python3 -m venv venv
source venv/bin/activate

# Install from source
pip install maturin
maturin develop --release --features python
```

## Quick Start

### Python Streaming Example

```python
import sdag

# Build a streaming DAG programmatically
builder = sdag.PyDagBuilder()
builder.add_input("price_a", 0)
builder.add_input("price_b", 1)
builder.add_add("total", "price_a", "price_b")
builder.add_constant("threshold", 100.0)
builder.add_comparison("alert", "total", "threshold", "GreaterThan")
builder.set_trigger("alert")  # Only emit when alert triggers
builder.set_outputs(["total"])

# Create engine
engine = builder.build()

# Stream data through the DAG
for row in price_stream:
    outputs = engine.evaluate_step([row.price_a, row.price_b])
    if outputs is not None:
        print(f"Alert! Total price: {outputs[0]}")
```

### YAML Definition

```yaml
nodes:
  - id: x
    type: Input
    params:
      input_index: 0
  - id: y
    type: Input
    params:
      input_index: 1
  - id: sum
    type: Add
    params:
      inputs: ["x", "y"]
  - id: threshold
    type: Constant
    params:
      value: 10.0
  - id: above_threshold
    type: Comparison
    params:
      inputs: ["sum", "threshold"]
      op: GreaterThan
trigger: above_threshold
outputs: ["sum"]
```

### Pure Rust Execution

```bash
# Run a DAG from YAML with command-line inputs
cargo run -- example.yaml 5.0 7.0
```

## Performance

The engine achieves exceptional performance through:
- Dense array storage for cache-friendly access
- Single enum dispatch (no vtables)
- Unsafe code in hot paths
- Incremental computation

See [PERFORMANCE.md](PERFORMANCE.md) for detailed benchmarks showing:
- 52M+ operations/second for small DAGs
- Sub-microsecond evaluation for 1000-node DAGs
- 1.4x speedup from incremental computation

## Architecture

The engine uses a data-oriented design with:
- All nodes stored in a single `Vec<NodeOp>` enum
- Parallel arrays for values, previous values, and change flags
- Topological ordering for correct evaluation
- Single-pass evaluation combining dirty marking and computation

## License

MIT