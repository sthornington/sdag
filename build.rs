fn main() {
    // Tell cargo to link as Python extension module (undefined dynamic lookup)
    pyo3_build_config::add_extension_module_link_args();
}
