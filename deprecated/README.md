# Deprecated Files

This folder contains files that were created during the refactoring process but are no longer needed.

## Test Files
- `test_simple.py` - Used to debug the freeze() method with constant nodes
- `debug_nodes.py` - Used to understand the Python node structure and field types
- `example_complete.py` - First attempt at complete example, had issues with finding node indices
- `example_working.py` - Second attempt, revealed the zero-value evaluation issue
- `test_freeze.py` - Initial debugging of the freeze() AttributeError
- `test_debug.py` - Debugging field extraction in freeze()
- `example_enhanced.py` - Attempted to show multi-engine support, had YAML parsing issues

## Unused Implementation Files
- `node_macro.rs` - Initial macro attempt, not used
- `define_node_macro.rs` - Second macro attempt, too complex
- `node_macro_v2.rs` - Third macro attempt with paste crate, compilation issues
- `nodes.rs`, `nodes_v2.rs` - Node implementations using the complex macros
- `engines.rs` - Separate engines file, integrated into engine.rs instead
- `engine_traits.rs` - Trait definitions, integrated into engine.rs
- `arena.rs` - Arena implementation, integrated into engine.rs
- `lib_v2.rs`, `lib_macro_attempt.rs` - Previous lib.rs versions with macro approaches

The main issue discovered: The evaluation engine expects nodes to be in topological order in the arena,
but when we change the root node for trigger-based evaluation, the order may no longer be correct.
This causes dependencies to be evaluated after their dependents, resulting in zero values.