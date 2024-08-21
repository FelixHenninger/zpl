use serde::Deserialize;
use zpl::{command::HostIdentification, label::Label};

use crate::configuration::LabelDimensions;

#[derive(Deserialize)]
pub struct PrintJob {}

impl PrintJob {
    pub fn into_label(
        &self,
        dim: &LabelDimensions,
        host: &HostIdentification,
    ) -> Label {
        let cwidth = ((dim.width - dim.margin_left - dim.margin_right).max(0.0)
            * (host.dpmm as f32)) as u32;
        let cheight = ((dim.height - dim.margin_top - dim.margin_bottom)
            .max(0.0)
            * (host.dpmm as f32)) as u32;

        let width = (dim.width * (host.dpmm as f32)) as u32;
        let height = (dim.height * (host.dpmm as f32)) as u32;
        let mut label = Label::new(width, height, host.dpmm);

        label
    }
}
