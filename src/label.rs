use std::path::PathBuf;

use anyhow::Context;

use crate::command::{self, CommandSequence, ZplCommand};

#[derive(Clone, Debug)]
pub enum LabelContent {
    Image {
        path: PathBuf,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    },
    Svg {
        path: PathBuf,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    },
}

#[derive(Clone)]
pub struct Label {
    pub content: Vec<LabelContent>,
}

impl Label {
    pub fn new() -> Self {
        Self { content: vec![] }
    }

    pub async fn render(
        &self,
        dpmm: u32,
    ) -> anyhow::Result<command::CommandSequence> {
        let mut output = CommandSequence(vec![]);

        for c in &self.content {
            match c {
                LabelContent::Image { path, x, y, w, h } => {
                    let img =
                        ::image::open(path).expect("Image file not found");
                    img.resize_to_fill(
                        *w * dpmm,
                        *h * dpmm,
                        ::image::imageops::FilterType::Lanczos3,
                    );

                    let img_serialized =
                        crate::image::SerializedImage::from_image(&img);

                    output.push(ZplCommand::MoveOrigin(*x * dpmm, *y * dpmm));
                    output.push(ZplCommand::RenderImage(img_serialized));
                }
                LabelContent::Svg { path, x, y, w, h } => {
                    let svg = tokio::fs::read_to_string(path)
                        .await
                        .expect("SVG file not found");

                    let img_serialized =
                        crate::image::SerializedImage::from_svg(
                            svg,
                            *w * dpmm,
                            *h * dpmm,
                        )
                        .context("Could not load SVG")?;

                    output.push(ZplCommand::MoveOrigin(*x * dpmm, *y * dpmm));
                    output.push(ZplCommand::RenderImage(img_serialized));
                }
            }
        }

        Ok(output)
    }
}
