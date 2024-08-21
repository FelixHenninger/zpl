use serde::Deserialize;
use zpl::{
    command::HostIdentification,
    label::{Label, LabelContent, Unit},
};

use crate::configuration::LabelDimensions;

#[derive(Deserialize)]
pub struct PrintJob {}

impl PrintJob {
    pub fn into_label(
        &self,
        dim: &LabelDimensions,
        host: &HostIdentification,
        code: String,
    ) -> Label {
        let cwidth = (dim.width - dim.margin_left - dim.margin_right).max(0.0);
        let cheight =
            (dim.height - dim.margin_top - dim.margin_bottom).max(0.0);

        let width = dim.width as u32;
        let height = dim.height as u32;
        let mut label = Label::new(width, height, host.dpmm);

        label.content.push(LabelContent::Svg {
            code,
            x: Unit::Millimetres(dim.margin_left),
            y: Unit::Millimetres(dim.margin_top),
            w: Unit::Millimetres(cwidth),
            h: Unit::Millimetres(cheight),
        });

        label
    }
}
