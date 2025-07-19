#!/bin/bash
set -e

# Build Rust library without Python
echo "Building Rust library..."
cargo build --release

# Build Python extension if requested
if [ "$1" == "python" ]; then
    echo "Building Python extension..."
    
    # Get Python config
    PYTHON_INCLUDE=$(python3 -c "from sysconfig import get_paths; print(get_paths()['include'])")
    PYTHON_LIB=$(python3 -c "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))")
    PYTHON_VERSION=$(python3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
    
    # Build with Python features
    RUSTFLAGS="-C link-arg=-undefined -C link-arg=dynamic_lookup" \
    cargo build --release --features python
    
    # Copy to Python module name
    if [ -f "target/release/libsdag.dylib" ]; then
        cp target/release/libsdag.dylib sdag.so
    elif [ -f "target/release/libsdag.so" ]; then
        cp target/release/libsdag.so sdag.so
    fi
    
    echo "Python extension built as sdag.so"
fi