#!/usr/bin/env python3
"""
Generate node definitions from nodes.yaml
"""
import yaml

def main():
    with open('nodes.yaml', 'r') as f:
        data = yaml.safe_load(f)
    
    nodes = data['nodes']
    
    # Generate the Rust code
    code = generate_rust_code(nodes)
    
    # Write to src/generated_nodes.rs
    with open('src/generated_nodes.rs', 'w') as f:
        f.write(code)
    
    print("Generated src/generated_nodes.rs")

def generate_rust_code(nodes):
    code = """// Auto-generated node definitions from nodes.yaml
// DO NOT EDIT - run generate_nodes.py to regenerate

use crate::simple_node_macro::{EvalNode, ArenaEval};
use crate::engine::{NodeId, ArenaNode, FieldValue};
use std::collections::HashMap;
use pyo3::prelude::*;

"""
    
    # Generate node structs and implementations
    for node in nodes:
        code += generate_node(node)
    
    # Generate Graph methods
    code += "\n// Graph builder methods\n"
    code += "#[pymethods]\n"
    code += "impl crate::Graph {\n"
    for node in nodes:
        code += generate_graph_method(node)
    code += "}\n\n"
    
    # Generate arena builder
    code += generate_arena_builder(nodes)
    
    # Generate Python registration
    code += generate_python_registration(nodes)
    
    # Generate freeze helper
    code += generate_freeze_helper(nodes)
    
    return code

def generate_node(node):
    name = node['name']
    tag = node['tag']
    fields = node.get('fields', {})
    eval_code = node['eval'].strip()
    
    code = f"// {name} node\n"
    code += f"#[derive(Debug, Clone)]\n"
    code += f"pub struct {name}Node {{\n"
    for field_name, field_type in fields.items():
        code += f"    pub {field_name}: {field_type},\n"
    code += "}\n\n"
    
    # EvalNode impl
    code += f"impl EvalNode for {name}Node {{\n"
    code += "    fn eval(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {\n"
    # Fix unused variables
    eval_fixed = eval_code
    if name == "Input":
        eval_fixed = eval_fixed.replace("values", "_values")
    elif name == "Const":
        eval_fixed = eval_fixed.replace("values", "_values").replace("inputs", "_inputs")
    else:
        eval_fixed = eval_fixed.replace("inputs", "_inputs")
    code += f"        {eval_fixed}\n"
    code += "    }\n"
    code += "}\n\n"
    
    # ArenaEval impl
    code += f"impl ArenaEval for {name}Node {{\n"
    code += "    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {\n"
    code += "        self.eval(values, inputs)\n"
    code += "    }\n"
    code += "}\n\n"
    
    # Python wrapper class
    code += "#[pyclass]\n"
    code += f"pub struct {name} {{\n"
    code += "    #[pyo3(get)]\n"
    code += "    pub id: String,\n"
    for field_name, field_type in fields.items():
        py_type = convert_to_py_type(field_type)
        code += "    #[pyo3(get)]\n"
        code += f"    pub {field_name}: {py_type},\n"
    code += "}\n\n"
    
    return code

def generate_graph_method(node):
    name = node['name']
    tag = node['tag']
    fields = node.get('fields', {})
    
    method_name = "const_" if tag == "const" else tag
    method_attr = '    #[pyo3(name = "const")]\n' if tag == "const" else ""
    
    code = f"{method_attr}    pub fn {method_name}(&mut self, py: Python"
    for field_name, field_type in fields.items():
        py_type = convert_to_py_type(field_type)
        code += f", {field_name}: {py_type}"
    code += ") -> PyObject {\n"
    code += "        let id = format!(\"n{}\", self.counter);\n"
    code += "        self.counter += 1;\n"
    code += f"        let node = {name} {{ id: id.clone()"
    for field_name in fields:
        code += f", {field_name}"
    code += " };\n"
    code += "        let py_node = node.into_py(py);\n"
    code += "        self.registry.insert(id, py_node.clone());\n"
    code += "        py_node\n"
    code += "    }\n\n"
    
    return code

def generate_arena_builder(nodes):
    code = """// Arena builder
pub fn build_arena_node(node: &ArenaNode) -> Result<Box<dyn ArenaEval>, String> {
    match node.tag.as_str() {
"""
    
    for node in nodes:
        name = node['name']
        tag = node['tag']
        fields = node.get('fields', {})
        
        code += f'        "{tag}" => {{\n'
        
        # Extract fields
        for field_name, field_type in fields.items():
            if field_type == "String":
                code += f'            let {field_name} = match node.fields.get("{field_name}") {{\n'
                code += f'                Some(FieldValue::Str(s)) => s.clone(),\n'
                code += f'                _ => return Err("node missing {field_name}".to_string()),\n'
                code += '            };\n'
            elif field_type == "f64":
                code += f'            let {field_name} = match node.fields.get("{field_name}") {{\n'
                code += f'                Some(FieldValue::Float(f)) => *f,\n'
                code += f'                _ => return Err("node missing {field_name}".to_string()),\n'
                code += '            };\n'
            elif field_type == "NodeId":
                code += f'            let {field_name} = match node.fields.get("{field_name}") {{\n'
                code += f'                Some(FieldValue::One(id)) => *id,\n'
                code += f'                _ => return Err("node missing {field_name}".to_string()),\n'
                code += '            };\n'
            elif field_type == "Vec<NodeId>":
                code += f'            let {field_name} = match node.fields.get("{field_name}") {{\n'
                code += f'                Some(FieldValue::Many(ids)) => ids.clone(),\n'
                code += f'                _ => return Err("node missing {field_name}".to_string()),\n'
                code += '            };\n'
        
        code += f'            Ok(Box::new({name}Node {{ '
        code += ', '.join(fields.keys())
        code += ' }))\n'
        code += '        },\n'
    
    code += '        _ => Err(format!("Unknown node type: {}", node.tag)),\n'
    code += '    }\n'
    code += '}\n\n'
    
    return code

def generate_python_registration(nodes):
    code = "// Python registration\n"
    code += "pub fn register_nodes(m: &pyo3::types::PyModule) -> PyResult<()> {\n"
    for node in nodes:
        code += f'    m.add_class::<{node["name"]}>()?;\n'
    code += "    Ok(())\n"
    code += "}\n\n"
    return code

def generate_freeze_helper(nodes):
    code = """// Freeze helper
pub fn freeze_node_fields(py: Python, obj: &PyObject, node_type: &str, mapping: &mut serde_yaml::Mapping, id2idx: &HashMap<String, usize>) -> PyResult<()> {
    use serde_yaml::Value;
    
    match node_type {
"""
    
    for node in nodes:
        tag = node['tag']
        fields = node.get('fields', {})
        
        code += f'        "{tag}" => {{\n'
        
        for field_name, field_type in fields.items():
            if field_type == "String":
                code += f'            let {field_name}: String = obj.as_ref(py).getattr("{field_name}")?.extract()?;\n'
                code += f'            mapping.insert(Value::String("{field_name}".into()), Value::String({field_name}));\n'
            elif field_type == "f64":
                code += f'            let {field_name}: f64 = obj.as_ref(py).getattr("{field_name}")?.extract()?;\n'
                code += f'            mapping.insert(Value::String("{field_name}".into()), serde_yaml::to_value({field_name}).unwrap());\n'
            elif field_type == "NodeId":
                code += f'            let {field_name}: PyObject = obj.as_ref(py).getattr("{field_name}")?.extract()?;\n'
                code += f'            let {field_name}_id: String = {field_name}.as_ref(py).getattr("id")?.extract()?;\n'
                code += f'            mapping.insert(Value::String("{field_name}".into()), Value::Number(serde_yaml::Number::from(id2idx[&{field_name}_id] as i64)));\n'
            elif field_type == "Vec<NodeId>":
                code += f'            let {field_name}: Vec<PyObject> = obj.as_ref(py).getattr("{field_name}")?.extract()?;\n'
                code += '            let mut idxs = Vec::new();\n'
                code += f'            for child in {field_name} {{\n'
                code += '                let cid: String = child.as_ref(py).getattr("id")?.extract()?;\n'
                code += '                idxs.push(Value::Number(serde_yaml::Number::from(id2idx[&cid] as i64)));\n'
                code += '            }\n'
                code += f'            mapping.insert(Value::String("{field_name}".into()), Value::Sequence(idxs));\n'
        
        code += '        },\n'
    
    code += '        _ => {},\n'
    code += '    }\n'
    code += '    Ok(())\n'
    code += '}\n'
    
    return code

def convert_to_py_type(rust_type):
    if rust_type == "NodeId":
        return "PyObject"
    elif rust_type == "Vec<NodeId>":
        return "Vec<PyObject>"
    else:
        return rust_type

if __name__ == "__main__":
    main()