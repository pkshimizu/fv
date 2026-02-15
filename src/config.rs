#[derive(Debug, Clone)]
pub struct Config {
    pub app_name: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_name: "fv".to_string(),
        }
    }
}
