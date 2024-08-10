use std::path::PathBuf;

use anyhow::Context;

use crate::command::{
    self, BackfeedSequence, CommandSequence, MediaTracking, MediaType,
    PostPrintAction, ZplCommand,
};

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
    pub width: u32,
    pub height: u32,
    pub dpmm: u32,
}

impl Label {
    pub fn new(width: u32, height: u32, dpmm: u32) -> Self {
        Self {
            content: vec![],
            width,
            height,
            dpmm,
        }
    }

    pub async fn render(&self) -> anyhow::Result<command::CommandSequence> {
        let mut output = CommandSequence(vec![]);

        for c in &self.content {
            match c {
                LabelContent::Image { path, x, y, w, h } => {
                    let img =
                        ::image::open(path).expect("Image file not found");
                    img.resize_to_fill(
                        *w * self.dpmm,
                        *h * self.dpmm,
                        ::image::imageops::FilterType::Lanczos3,
                    );

                    let img_serialized =
                        crate::image::SerializedImage::from_image(&img);

                    output.push(ZplCommand::MoveOrigin(
                        *x * self.dpmm,
                        *y * self.dpmm,
                    ));
                    output.push(ZplCommand::RenderImage(img_serialized));
                }
                LabelContent::Svg { path, x, y, w, h } => {
                    let svg = tokio::fs::read_to_string(path)
                        .await
                        .expect("SVG file not found");

                    let img_serialized =
                        crate::image::SerializedImage::from_svg(
                            svg,
                            *w * self.dpmm,
                            *h * self.dpmm,
                        )
                        .context("Could not load SVG")?;

                    output.push(ZplCommand::MoveOrigin(
                        *x * self.dpmm,
                        *y * self.dpmm,
                    ));
                    output.push(ZplCommand::RenderImage(img_serialized));
                }
            }
        }

        Ok(output)
    }

    pub async fn print(&self, copies: u32) -> anyhow::Result<CommandSequence> {
        let mut commands = make_preamble();
        commands.append(CommandSequence(vec![
            ZplCommand::StartLabel,
            ZplCommand::SetPostPrintAction(PostPrintAction::Cut),
            ZplCommand::SetPrintWidth(self.width * self.dpmm),
            ZplCommand::SetLabelLength(self.height * self.dpmm),
            ZplCommand::SetHorizontalShift(0),
        ]));
        commands.append(self.render().await?);
        commands.append(CommandSequence(vec![
            ZplCommand::PrintQuantity {
                total: copies,
                pause_and_cut_after: copies,
                replicates_per_serial: copies,
                cut_only: true,
            },
            ZplCommand::EndLabel,
        ]));

        Ok(commands)
    }
}

pub fn make_preamble() -> CommandSequence {
    CommandSequence(vec![
        ZplCommand::SetDelimiter(','),
        ZplCommand::SetControlCommandPrefix('~'),
        ZplCommand::SetFormatCommandPrefix('^'),
        ZplCommand::SetEncoding(0),
        ZplCommand::StartLabel,
        ZplCommand::SetTearOffPosition(0),
        ZplCommand::SetVerticalShift(0),
        ZplCommand::SetMediaType(MediaType::Transfer),
        ZplCommand::SetMediaTracking(MediaTracking::NonContinuousWebSensing),
        ZplCommand::SetBackfeedSequence(BackfeedSequence::Default),
        ZplCommand::SetHome(0, 0),
        ZplCommand::SetDarkness(25),
        ZplCommand::SetHalfDensity(false),
        ZplCommand::SetSpeed { print: 4, slew: 4 },
        ZplCommand::PersistConfiguration,
        ZplCommand::SetInverted(false),
        // Adjustments
        ZplCommand::SetVerticalShift(12),
        ZplCommand::SetTearOffPosition(-20),
        ZplCommand::EndLabel,
    ])
}
