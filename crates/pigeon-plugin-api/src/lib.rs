//! Plugin host trait surface. WIT bindings + wasmtime runtime arrive in M14.

pub trait PluginHost {
    fn name(&self) -> &str;
}
