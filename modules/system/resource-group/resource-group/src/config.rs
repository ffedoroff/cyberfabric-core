use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourceGroupConfig {
    #[serde(default = "default_url_prefix")]
    pub url_prefix: String,

    #[serde(default)]
    pub max_depth: Option<usize>,

    #[serde(default)]
    pub max_width: Option<usize>,
}

fn default_url_prefix() -> String {
    "/api/resource-group".to_owned()
}

impl Default for ResourceGroupConfig {
    fn default() -> Self {
        Self {
            url_prefix: default_url_prefix(),
            max_depth: None,
            max_width: None,
        }
    }
}
