use serde::{Deserialize, Serialize};

use std::{collections::HashMap, net::SocketAddr, path::Path};

#[derive(Deserialize, Serialize)]
pub struct Configuration {
    pub labels: HashMap<LabelIdentifier, Shared<Label>>,
    pub printers: HashMap<String, Shared<LabelPrinter>>,
}

pub struct Shared<T>(pub std::sync::Arc<T>);

#[derive(Deserialize, Serialize)]
pub struct Label {
    pub dimensions: LabelDimensions,
}

/// Identifies a label type.
#[derive(Deserialize, Serialize, Clone, Eq, PartialEq, Hash)]
pub struct LabelIdentifier(pub String);

/// Identifies a logical printer, i.e. one the server opens a connection to.
#[derive(Deserialize, Serialize, Clone, Eq, PartialEq, Hash)]
pub struct PrinterIdentifier(pub String);

#[derive(Deserialize, Serialize)]
pub struct LabelPrinter {
    pub label: LabelIdentifier,
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

impl<'de, T> Deserialize<'de> for Shared<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        Ok(Shared(std::sync::Arc::new(value)))
    }
}

impl<T> Serialize for Shared<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        T::serialize(self.0.as_ref(), serializer)
    }
}
