use serde::Deserialize;
use zpl::{
    command::HostIdentification,
    label::{Label, LabelContent, Unit},
};

use crate::{configuration::LabelDimensions, data_uri::DataUri};

#[derive(Deserialize)]
#[non_exhaustive]
pub enum PrintApi {
    #[serde(rename = "svg")]
    #[non_exhaustive]
    Svg { code: String },
    #[serde(rename = "image")]
    #[non_exhaustive]
    Image { data: DataUri },
}

#[non_exhaustive]
pub enum PrintJob {
    Svg { code: String },
    Image { image: image::DynamicImage },
}

impl PrintApi {
    pub fn validate_as_job(&self) -> anyhow::Result<PrintJob> {
        Ok(match self {
            // FIXME: should validate here.
            PrintApi::Svg { code } => PrintJob::Svg { code: code.clone() },
            PrintApi::Image { data: uri } => {
                let data = std::io::Cursor::new(uri.data.clone());
                let image = {
                    let mut reader = image::io::Reader::new(data);

                    let format_hint = match uri.mime.as_str() {
                        "image/png" | "application/png" => {
                            Some(image::ImageFormat::Png)
                        }
                        "image/jpg" | "image/jpeg" => {
                            Some(image::ImageFormat::Jpeg)
                        }
                        other => {
                            eprintln!("Unknown image format {other}");
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
            PrintJob::Svg { code } => {
                label.content.push(LabelContent::Svg {
                    code: code.clone(),
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
        }

        label
    }
}
