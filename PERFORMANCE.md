# Performance Benchmarks

The high-performance streaming DAG engine achieves exceptional performance through:
- Dense array storage for cache-friendly memory access
- Single enum dispatch (no vtables or trait objects)
- Unsafe code in hot paths for zero-cost array access
- Incremental computation that only recomputes changed paths

## Benchmark Results

All benchmarks run on streaming evaluation with trigger-based output.

### Linear DAG Performance
Chain of add operations: `input -> +1 -> +2 -> ... -> +N`

| Size | Time per Evaluation | Operations/Second |
|------|-------------------|------------------|
| 10 nodes | 19ns | 52.6M ops/sec |
| 100 nodes | 140ns | 7.1M ops/sec |
| 1000 nodes | 1.4µs | 714K ops/sec |

### Wide DAG Performance  
Many inputs summed together: `sum(input1, input2, ..., inputN)`

| Width | Time per Evaluation | Operations/Second |
|-------|-------------------|------------------|
| 10 inputs | 17ns | 58.8M ops/sec |
| 50 inputs | 65ns | 15.4M ops/sec |
| 100 inputs | 128ns | 7.8M ops/sec |

### Diamond DAG Performance
Complex dependency patterns with multiple paths

| Depth | Time per Evaluation | Operations/Second |
|-------|-------------------|------------------|
| 5 layers | 18ns | 55.6M ops/sec |
| 10 layers | 27ns | 37.0M ops/sec |
| 15 layers | 35ns | 28.6M ops/sec |

### Incremental Update Performance
DAG with 10 independent computation chains

| Scenario | Time per Update | Speedup vs Full |
|----------|----------------|-----------------|
| 1 input changed | 43ns | 1.4x faster |
| All inputs changed | 60ns | baseline |

## Key Performance Features

1. **Zero-allocation evaluation**: After initial setup, no heap allocations during evaluation
2. **Cache-friendly layout**: All node data in contiguous arrays
3. **Predictable performance**: No GC pauses or dynamic dispatch overhead
4. **Incremental computation**: Only recomputes nodes affected by changes
5. **SIMD-friendly**: Data layout enables potential future SIMD optimizations

## Running Benchmarks

```bash
cargo bench
```

This will generate detailed benchmark reports in `target/criterion/`.