pub trait LocationResolver {
    type Location;

    fn resolve(&self, path_template: &str, entity_json: &serde_json::Value) -> Self::Location;
    fn base_of(&self, location: &Self::Location) -> String;
}
