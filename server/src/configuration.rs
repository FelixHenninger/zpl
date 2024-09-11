use serde::{Deserialize, Serialize};

use std::{collections::HashMap, net::SocketAddr, path::Path, sync::Arc};

#[derive(Deserialize, Serialize)]
pub struct Configuration {
    pub labels: HashMap<LabelIdentifier, Arc<Label>>,
    pub printers: HashMap<String, Arc<LabelPrinter>>,
}

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

impl LabelDimensions {
    /// Compare sizes, approximately considering json serialization semantics on either side of
    /// Rust or HTML / JS may not be exactly the same. That is, only compare like 5 digits which is
    /// far too good for measurement anyhow.
    pub fn approx_cmp(&self, other: &Self) -> bool {
        fn to_5digits(lhs: f32, rhs: f32) -> Option<core::cmp::Ordering> {
            // Can underflow but that's fine. An 'eps' of 0.0 is just a very harsh requirement.
            let eps = lhs.abs().max(rhs.abs()) * 2.0f32.powi(-16);
            let diff = lhs - rhs;

            if diff.abs() <= eps {
                Some(core::cmp::Ordering::Equal)
            } else if diff < 0.0 {
                Some(core::cmp::Ordering::Less)
            } else if diff > 0.0 {
                Some(core::cmp::Ordering::Greater)
            } else {
                None
            }
        }

        fn to_5digits_eq(lhs: f32, rhs: f32) -> bool {
            matches!(to_5digits(lhs, rhs), Some(core::cmp::Ordering::Equal))
        }

        let LabelDimensions {
            width,
            height,
            margin_left,
            margin_right,
            margin_top,
            margin_bottom,
        } = *self;

        to_5digits_eq(width, other.width)
            && to_5digits_eq(height, other.height)
            && to_5digits_eq(margin_left, other.margin_left)
            && to_5digits_eq(margin_right, other.margin_right)
            && to_5digits_eq(margin_top, other.margin_top)
            && to_5digits_eq(margin_bottom, other.margin_bottom)
    }
}

#[test]
fn validate_dimensions() {
    let lhs = LabelDimensions {
        width: 51.,
        height: 51.,
        margin_left: 1.,
        margin_right: 1.,
        margin_top: 1.,
        margin_bottom: 1.,
    };

    assert!(lhs.approx_cmp(&lhs), "Does not equal itself?");

    assert!(
        !LabelDimensions { width: 50., ..lhs }.approx_cmp(&lhs),
        "Does equal another"
    );

    assert!(
        !LabelDimensions { height: 50., ..lhs }.approx_cmp(&lhs),
        "Does equal another"
    );
}
