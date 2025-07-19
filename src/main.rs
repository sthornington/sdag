use std::env;
use std::fs;
use anyhow::Result;
use sdag::engine;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <dag.yaml> [input_values...]", args[0]);
        eprintln!("Example: {} example.yaml 1.0 2.0 3.0", args[0]);
        std::process::exit(1);
    }

    let yaml_content = fs::read_to_string(&args[1])?;
    let mut engine = engine::from_yaml(&yaml_content)?;
    
    println!("Loaded DAG from {}", args[1]);
    
    // Parse input values if provided
    let input_values: Vec<f64> = if args.len() > 2 {
        args[2..].iter()
            .map(|s| s.parse::<f64>())
            .collect::<Result<Vec<_>, _>>()?
    } else {
        // Default to zeros
        vec![0.0; 10] // Support up to 10 inputs
    };
    
    println!("Evaluating with inputs: {:?}", input_values);
    
    // Run one evaluation step
    if let Some(outputs) = engine.evaluate_step(&input_values) {
        println!("\nTrigger fired! Outputs: {:?}", outputs);
    } else {
        println!("\nNo trigger fired");
    }
    
    // Show all node values
    println!("\nAll node values:");
    for (i, val) in engine.get_all_values().iter().enumerate() {
        println!("  Node {}: {}", i, val);
    }
    
    Ok(())
}
