use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use sdag::{Engine, NodeOp, ComparisonOp};

fn create_linear_dag(size: usize) -> Vec<NodeOp> {
    let mut nodes = Vec::with_capacity(size + 2);
    
    // Input node
    nodes.push(NodeOp::Input { input_index: 0 });
    
    // Chain of additions with constants
    for i in 1..=size {
        nodes.push(NodeOp::Constant(i as f64));
        nodes.push(NodeOp::Add { a: nodes.len() - 2, b: nodes.len() - 1 });
    }
    
    nodes
}

fn create_wide_dag(width: usize) -> Vec<NodeOp> {
    let mut nodes = Vec::new();
    
    // Input nodes
    for i in 0..width {
        nodes.push(NodeOp::Input { input_index: i });
    }
    
    // Sum all inputs
    nodes.push(NodeOp::Sum { inputs: (0..width).collect() });
    
    // Add a trigger
    nodes.push(NodeOp::Constant(100.0));
    nodes.push(NodeOp::Comparison { 
        a: nodes.len() - 2, 
        b: nodes.len() - 1, 
        op: ComparisonOp::GreaterThan 
    });
    
    nodes
}

fn create_diamond_dag(depth: usize) -> Vec<NodeOp> {
    let mut nodes = Vec::new();
    
    // Two inputs
    nodes.push(NodeOp::Input { input_index: 0 });
    nodes.push(NodeOp::Input { input_index: 1 });
    
    // Create diamond pattern
    let mut layer_start = 0;
    let mut layer_size = 2;
    
    for _ in 0..depth {
        let prev_layer_start = layer_start;
        let prev_layer_size = layer_size;
        layer_start = nodes.len();
        
        // Each node in new layer depends on two nodes from previous layer
        for i in 0..prev_layer_size {
            let a = prev_layer_start + i;
            let b = prev_layer_start + (i + 1) % prev_layer_size;
            nodes.push(NodeOp::Add { a, b });
        }
        
        layer_size = prev_layer_size;
    }
    
    // Final sum
    let final_inputs: Vec<_> = (layer_start..nodes.len()).collect();
    nodes.push(NodeOp::Sum { inputs: final_inputs });
    
    nodes
}

fn bench_linear_dag(c: &mut Criterion) {
    let mut group = c.benchmark_group("linear_dag");
    
    for size in [10, 100, 1000].iter() {
        let nodes = create_linear_dag(*size);
        let mut engine = Engine::new(nodes);
        engine.set_outputs(vec![engine.get_all_values().len() - 1]);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size), 
            size, 
            |b, _| {
                b.iter(|| {
                    engine.evaluate_step(black_box(&[1.0]))
                })
            },
        );
    }
    
    group.finish();
}

fn bench_wide_dag(c: &mut Criterion) {
    let mut group = c.benchmark_group("wide_dag");
    
    for width in [10, 50, 100].iter() {
        let nodes = create_wide_dag(*width);
        let mut engine = Engine::new(nodes);
        let sum_node = engine.get_all_values().len() - 3;
        engine.set_trigger(engine.get_all_values().len() - 1);
        engine.set_outputs(vec![sum_node]);
        
        let inputs = vec![1.0; *width];
        
        group.bench_with_input(
            BenchmarkId::from_parameter(width), 
            width, 
            |b, _| {
                b.iter(|| {
                    engine.evaluate_step(black_box(&inputs))
                })
            },
        );
    }
    
    group.finish();
}

fn bench_diamond_dag(c: &mut Criterion) {
    let mut group = c.benchmark_group("diamond_dag");
    
    for depth in [5, 10, 15].iter() {
        let nodes = create_diamond_dag(*depth);
        let mut engine = Engine::new(nodes);
        engine.set_outputs(vec![engine.get_all_values().len() - 1]);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(depth), 
            depth, 
            |b, _| {
                b.iter(|| {
                    engine.evaluate_step(black_box(&[1.0, 2.0]))
                })
            },
        );
    }
    
    group.finish();
}

fn bench_incremental_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_updates");
    
    // Create a DAG with multiple independent paths
    let mut nodes = Vec::new();
    
    // 10 input nodes
    for i in 0..10 {
        nodes.push(NodeOp::Input { input_index: i });
    }
    
    // Create 10 independent computation chains
    for i in 0..10 {
        nodes.push(NodeOp::Constant(2.0));
        nodes.push(NodeOp::Multiply { a: i, b: nodes.len() - 1 }); // input * 2
        nodes.push(NodeOp::Constant(10.0));
        nodes.push(NodeOp::Add { a: nodes.len() - 2, b: nodes.len() - 1 }); // + 10
    }
    
    let mut engine = Engine::new(nodes);
    
    // First run to warm up
    let all_changed = vec![1.0; 10];
    engine.evaluate_step(&all_changed);
    
    // Benchmark changing only one input
    let mut one_changed = vec![1.0; 10];
    group.bench_function("one_input_changed", |b| {
        b.iter(|| {
            one_changed[0] += 0.1;
            engine.evaluate_step(black_box(&one_changed))
        })
    });
    
    // Benchmark changing all inputs
    let mut all_inputs = vec![1.0; 10];
    group.bench_function("all_inputs_changed", |b| {
        b.iter(|| {
            for i in 0..10 {
                all_inputs[i] += 0.1;
            }
            engine.evaluate_step(black_box(&all_inputs))
        })
    });
    
    group.finish();
}

criterion_group!(
    benches, 
    bench_linear_dag,
    bench_wide_dag,
    bench_diamond_dag,
    bench_incremental_updates
);
criterion_main!(benches);