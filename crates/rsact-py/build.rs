fn main() {
    // Needed so `stub_gen` (a plain binary, no `pyo3/extension-module`) links
    // against libpython correctly on platforms that require it (macOS in
    // particular). maturin passes `--features pyo3/extension-module` for the
    // actual `rsact` cdylib itself, where this is a no-op.
    pyo3_build_config::add_extension_module_link_args();
}
