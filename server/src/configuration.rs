use serde::{Deserialize, Serialize};

use std::{collections::HashMap, net::SocketAddr, path::Path};

#[derive(Deserialize, Serialize)]
pub struct Configuration {
    pub labels: HashMap<String, LabelPrinter>,
}

#[derive(Deserialize, Serialize)]
pub struct LabelPrinter {
    pub dimensions: LabelDimensions,
    pub addr: SocketAddr,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct LabelDimensions {
    pub width: f32,
    pub height: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    pub margin_top: f32,
    pub margin_bottom: f32,
}

impl Configuration {
    pub async fn from_file(path: &Path) -> anyhow::Result<Self> {
        let data = tokio::fs::read(path).await?;
        Ok(serde_json::de::from_slice(&data)?)
    }
}
