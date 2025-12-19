use serde::{Deserialize, Serialize};

use std::{collections::HashMap, net::SocketAddr, path::Path, sync::Arc};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub labels: HashMap<LabelIdentifier, Arc<Label>>,
    pub printers: HashMap<String, Arc<LabelPrinter>>,
    /// Root directory for typst files that may be imported.
    pub typst_root: Option<std::path::PathBuf>,
}

impl Configuration {
    pub async fn from_file(path: &Path) -> anyhow::Result<Self> {
        let data = tokio::fs::read(path).await?;
        Ok(serde_json::de::from_slice(&data)?)
    }
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
#[serde(deny_unknown_fields)]
pub struct LabelPrinter {
    /// How to refer to this printer in the server.
    pub label: LabelIdentifier,

    /// How to reach this printer?
    pub addr: SocketAddr,

    /// How to refer to this printer for the user.
    #[serde(default)]
    pub display_name: Option<String>,

    #[serde(default)]
    pub calibration: Option<LabelCalibration>,

    #[serde(default)]
    /// If this is not a physical printer, how do we handle it?
    pub virtualization: LabelVirtualization,

    #[serde(default)]
    pub connection: PrinterConnectionSettings,
}

#[derive(Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum LabelVirtualization {
    /// Connect to the printer, and print jobs.
    #[default]
    Physical,
    /// Instead of handling jobs into labels, just wait. Still, create a connection to the printer.
    DropJobs {
        persist: Option<std::path::PathBuf>,
        wait_time: std::time::Duration,
    },
    /// FIXME: not sure, don't rely on a connection at all but still drop jobs.
    ZplOnly {
        dpmm: Option<u32>,
        persist: Option<std::path::PathBuf>,
        wait_time: std::time::Duration,
    },
}

#[derive(Deserialize, Serialize)]
pub struct LabelDimensions {
    /// Width of the label in mm.
    pub width: f32,
    /// Height of the label in mm.
    pub height: f32,
    /// Space to reserve on the left of the label, as by printed direction.
    pub margin_left: f32,
    /// Space to reserve on the right of the label, as by printed direction.
    pub margin_right: f32,
    /// Space to reserve on top of the label, as by printed direction.
    pub margin_top: f32,
    /// Space to reserve at the bottom of the label, as by printed direction.
    pub margin_bottom: f32,
}

#[derive(Default, Deserialize, Serialize)]
pub struct LabelCalibration {
    /// Offset of the label towards the right (positive width) in mm.
    pub home_x: f32,
}

#[derive(Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "snake_case")]
pub struct PrinterConnectionSettings {
    pub retry_fail: std::time::Duration,

    /// Within an active interval, how often to query device status?
    pub status_report_interval: std::time::Duration,

    /// When no job is active and the connection is dropped, how often to look for the device
    /// anyways?
    pub disinterest_probe_interval: std::time::Duration,

    /// How long to wait when we try to establish a connection?
    pub connect_timeout: std::time::Duration,

    /// How long to wait with nothing todo until we drop the connection?
    pub idle_timeout: std::time::Duration,
}

impl Default for PrinterConnectionSettings {
    fn default() -> Self {
        PrinterConnectionSettings {
            retry_fail: std::time::Duration::from_secs(1),
            status_report_interval: std::time::Duration::from_secs(10),
            disinterest_probe_interval: std::time::Duration::from_secs(60),
            connect_timeout: std::time::Duration::from_secs(1),
            idle_timeout: std::time::Duration::from_secs(60),
        }
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

impl LabelVirtualization {
    pub fn is_connnected(&self) -> bool {
        matches!(
            self,
            LabelVirtualization::Physical
                | LabelVirtualization::DropJobs { .. },
        )
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
