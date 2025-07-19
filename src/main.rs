use std::env;
use std::fs;
use std::sync::Arc;
use anyhow::Result;
use sdag::{NodeRegistry, yaml::load_dag_from_yaml};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <dag.yaml>", args[0]);
        std::process::exit(1);
    }

    let yaml_content = fs::read_to_string(&args[1])?;
    let registry = Arc::new(NodeRegistry::new());
    let dag = load_dag_from_yaml(&yaml_content, registry)?;
    
    println!("Executing DAG from {}", args[1]);
    let results = dag.execute()?;
    
    println!("\nResults:");
    for (node_id, outputs) in results {
        println!("  Node '{}': {:?}", node_id, outputs);
    }
    
    Ok(())
}
