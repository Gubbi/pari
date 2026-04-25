/// Turns a path template plus entity JSON into a backend-specific
/// location. Backends decide the `Location` type (e.g. `PathBuf`).
pub trait LocationResolver {
    type Location;

    fn resolve(&self, path_template: &str, entity_json: &serde_json::Value) -> Self::Location;
    fn base_of(&self, location: &Self::Location) -> String;
}
