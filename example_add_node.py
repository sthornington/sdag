#!/usr/bin/env python3
"""
Example showing how simple it is to add a new node type.

To add a new node, you just need to:
1. Define the node struct with its fields
2. Implement the EvalNode trait with eval() method
3. Add the Python wrapper class 
4. Add a method to Graph to create the node
5. Handle it in the Sampler's node building

That's it! No complex macros or boilerplate.
"""

# This is what you'd add to lib.rs:

"""
// Max node - returns the maximum of its children
#[derive(Debug, Clone)]
pub struct MaxNode {
    pub children: Vec<NodeId>,
}

impl EvalNode for MaxNode {
    fn eval(&self, values: &[f64], _inputs: &HashMap<String, f64>) -> f64 {
        self.children.iter()
            .map(|&id| values[id])
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

impl ArenaEval for MaxNode {
    fn eval_arena(&self, values: &[f64], inputs: &HashMap<String, f64>) -> f64 {
        self.eval(values, inputs)
    }
}

// Python wrapper
#[pyclass]
pub struct Max {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub children: Vec<PyObject>,
}

// Add to Graph implementation:
fn max(&mut self, py: Python, children: Vec<PyObject>) -> PyObject {
    let id = format!("n{}", self.counter);
    self.counter += 1;
    let node = Max { id: id.clone(), children };
    let py_node = node.into_py(py);
    self.registry.insert(id, py_node.clone());
    py_node
}

// Add to Sampler's node building:
"max" => {
    let children = match arena_node.fields.get("children") {
        Some(engine::FieldValue::Many(ids)) => ids.clone(),
        _ => return Err(pyo3::exceptions::PyValueError::new_err("max node missing children")),
    };
    Box::new(MaxNode { children })
},

// Add to freeze_graph's node type matching:
"Max" => {
    let children: Vec<PyObject> = obj.as_ref(py).getattr("children")?.extract()?;
    let mut idxs = Vec::new();
    for child in children {
        let cid: String = child.as_ref(py).getattr("id")?.extract()?;
        idxs.push(Value::Number(serde_yaml::Number::from(id2idx[&cid] as i64)));
    }
    mapping.insert(Value::String("children".into()), Value::Sequence(idxs));
},

// Add to freeze_graph's tag matching:
"Max" => "max",

// Add to PyModule:
m.add_class::<Max>()?;
"""

print("To add a new node type, it's just a few lines of code!")
print("No complex macros, no boilerplate generation.")
print("Just define the struct, implement eval(), and add the Python bindings.")