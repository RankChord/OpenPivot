/// Plugin trait — the core interface all plugins must implement.
/// Plugins are loaded as dylibs and this trait provides the registration surface.
#[async_trait::async_trait]
pub trait Plugin: Send + Sync + std::fmt::Debug {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    
    /// Called when plugin is loaded
    fn on_load(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    
    /// Called when plugin is unloaded
    fn on_unload(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
