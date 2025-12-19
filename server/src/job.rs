use std::sync::{Arc, OnceLock};
use tokio::sync::watch;

use log::error;
use serde::Deserialize;

use zpl::{
    command::HostIdentification,
    label::{Label, LabelContent, Unit},
    resvg::{usvg, usvg::fontdb},
};

use crate::{configuration::LabelDimensions, data_uri::DataUri};

#[derive(Deserialize)]
pub struct PrintApi {
    /// A requirement for the label dimensions.
    ///
    /// The server is to validate that the printer addressed with this job has the same label
    /// dimensions as given. This lets us mitigate a 'race condition' where the printer is
    /// physically reconfigured and reloaded without it being given a new name.
    pub dimensions: Option<LabelDimensions>,
    #[serde(flatten)]
    pub kind: PrintApiKind,
}

#[derive(Deserialize)]
#[non_exhaustive]
pub enum PrintApiKind {
    #[serde(rename = "svg")]
    #[non_exhaustive]
    Svg { code: String },
    #[serde(rename = "image")]
    #[non_exhaustive]
    Image { data: DataUri },
    #[serde(rename = "typst")]
    #[non_exhaustive]
    Typst { code: String },
}

impl PrintApi {
    pub fn validate_as_job(&self) -> anyhow::Result<PrintJob> {
        Ok(match &self.kind {
            PrintApiKind::Svg { code } => {
                let tree = usvg::Tree::from_str(&code, Self::svg_options())?;
                PrintJob::Svg { tree }
            }
            PrintApiKind::Typst { code } => {
                PrintJob::Typst { code: code.clone() }
            }
            PrintApiKind::Image { data: uri } => {
                let data = std::io::Cursor::new(uri.data.clone());
                let image = {
                    let mut reader = image::ImageReader::new(data);

                    let format_hint = match uri.mime.as_str() {
                        "image/png" | "application/png" => {
                            Some(image::ImageFormat::Png)
                        }
                        "image/jpg" | "image/jpeg" => {
                            Some(image::ImageFormat::Jpeg)
                        }
                        other => {
                            error!("Unknown image format {other}");
                            None
                        }
                    };

                    if let Some(format) = format_hint {
                        reader.set_format(format);
                    }

                    reader.decode()?
                };

                PrintJob::Image { image }
            }
        })
    }

    /// Get SVG parsing and rendering options for usvg / resvg.
    ///
    /// Keep in mind this is one choice. It's not clear if this should be a static and if not,
    /// which object should keep the authoritative version and how to refresh them. But in
    /// particular using the system font database introduces a hard host-dependency that is
    /// implicit. For stronger reproducibility / fingerprint resistance (okay maybe that concern is
    /// hardly realistic) it would be better to have an explicit list shared by the host
    /// environment or at least allowing it to override. And then if we combine that with
    /// hot-reloading we get fully dynamic state that we nevertheless want to share between labels
    /// being printed.
    fn svg_options() -> &'static usvg::Options<'static> {
        static ONCE: OnceLock<usvg::Options> = OnceLock::new();
        ONCE.get_or_init(|| {
            let mut db = fontdb::Database::new();
            db.load_system_fonts();

            usvg::Options {
                fontdb: db.into(),
                ..Default::default()
            }
        })
    }
}

/// The representation after ingestion by the API. We try to avoid IO, in particular fallible IO,
/// after that representation has been reached. This reduces the number of late errors that must
/// wait on a device to process the job to be noticed.
#[non_exhaustive]
pub enum PrintJob {
    Svg { tree: usvg::Tree },
    Image { image: image::DynamicImage },
    Typst { code: String },
}

impl PrintJob {
    pub fn into_label(
        self,
        dim: &LabelDimensions,
        host: &HostIdentification,
    ) -> Label {
        let cwidth = (dim.width - dim.margin_left - dim.margin_right).max(0.0);
        let cheight =
            (dim.height - dim.margin_top - dim.margin_bottom).max(0.0);

        let width = dim.width as u32;
        let height = dim.height as u32;
        let mut label = Label::new(width, height, host.dpmm);

        match self {
            PrintJob::Svg { tree } => {
                label.content.push(LabelContent::SvgTree {
                    tree,
                    x: Unit::Millimetres(dim.margin_left),
                    y: Unit::Millimetres(dim.margin_top),
                    w: Unit::Millimetres(cwidth),
                    h: Unit::Millimetres(cheight),
                });
            }
            PrintJob::Image { image } => {
                label.content.push(LabelContent::Image {
                    img: image,
                    x: Unit::Millimetres(dim.margin_left),
                    y: Unit::Millimetres(dim.margin_top),
                    w: Unit::Millimetres(cwidth),
                    h: Unit::Millimetres(cheight),
                });
            }
            PrintJob::Typst { code } => {
                label.content.push(LabelContent::Typst { code });
            }
        }

        label
    }
}

/// A resource that can be monitored for interest by other tasks.
///
/// Clones refer to the same underlying resource.
#[derive(Clone)]
pub struct Interest {
    available: Arc<watch::Sender<usize>>,
}

pub struct KeepInterest {
    available: Arc<watch::Sender<usize>>,
}

impl Default for Interest {
    fn default() -> Self {
        Self {
            available: Arc::new(watch::Sender::new(0)),
        }
    }
}

impl Interest {
    /// Generate a token that signals interest in this resource while it is alive.
    pub fn uphold(&self) -> KeepInterest {
        self.available.send_modify(|n| *n += 1);

        KeepInterest {
            available: self.available.clone(),
        }
    }

    /// Check if, right now, there is at least one interested party.
    pub fn has_strong_interest(&self) -> bool {
        *self.available.borrow() > 0
    }

    /// Wait until there is at least one interested party.
    pub async fn wait_for_strong_interest(&self) -> bool {
        let mut recv = self.available.subscribe();
        let result = recv.wait_for(|n| *n > 0).await;
        result.is_ok()
    }
}

impl Drop for KeepInterest {
    fn drop(&mut self) {
        self.available.send_modify(|n| *n -= 1);
    }
}
