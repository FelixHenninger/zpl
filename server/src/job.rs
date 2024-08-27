use serde::Deserialize;
use zpl::{
    command::HostIdentification,
    label::{Label, LabelContent, Unit},
};

use crate::{configuration::LabelDimensions, data_uri::DataUri};

#[derive(Deserialize)]
#[non_exhaustive]
pub enum PrintJob {
    #[serde(rename = "svg")]
    #[non_exhaustive]
    Svg { code: String },
    #[serde(rename = "image")]
    #[non_exhaustive]
    Image { data: DataUri },
}

impl PrintJob {
    pub fn into_label(
        &self,
        dim: &LabelDimensions,
        host: &HostIdentification,
    ) -> anyhow::Result<Label> {
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
            PrintJob::Image { data: uri } => {
                let data = std::io::Cursor::new(uri.data.clone());
                let img = image::io::Reader::new(data).decode()?;
                label.content.push(LabelContent::Image {
                    img,
                    x: Unit::Millimetres(dim.margin_left),
                    y: Unit::Millimetres(dim.margin_top),
                    w: Unit::Millimetres(cwidth),
                    h: Unit::Millimetres(cheight),
                });
            }
        }

        Ok(label)
    }
}
