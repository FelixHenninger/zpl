use anyhow::Context;

use crate::command::{
    self, BackfeedSequence, CommandSequence, MediaTracking, MediaType,
    PostPrintAction, ZplCommand,
};

#[derive(Clone, Debug)]
pub enum LabelContent {
    Image {
        img: ::image::DynamicImage,
        x: Unit,
        y: Unit,
        w: Unit,
        h: Unit,
    },
    Svg {
        code: String,
        x: Unit,
        y: Unit,
        w: Unit,
        h: Unit,
    },
    SvgTree {
        tree: resvg::usvg::Tree,
        x: Unit,
        y: Unit,
        w: Unit,
        h: Unit,
    },
    QrCode {
        content: String,
        x: Unit,
        y: Unit,
        zoom: u32,
    },
}

#[derive(Clone, Debug)]
pub enum Unit {
    Dots(u32),
    Millimetres(f32),
}

#[derive(Clone)]
pub struct Label {
    pub content: Vec<LabelContent>,
    pub width: u32,
    pub height: u32,
    pub dpmm: u32,
}

#[derive(Default)]
pub struct PrintOptions {
    pub copies: u32,
    pub calibration: Option<PrintCalibration>,
}

pub struct PrintCalibration {
    pub home_x: Unit,
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

    pub fn unit_to_dots(&self, u: &Unit) -> u32 {
        match u {
            Unit::Dots(d) => *d,
            Unit::Millimetres(mm) => (mm * self.dpmm as f32).floor() as u32,
        }
    }

    pub fn signed_unit_to_dots(&self, u: &Unit) -> i32 {
        match u {
            &Unit::Dots(d) => d.try_into().unwrap_or(i32::MAX),
            // conversion from float to integer is specified to clamp.
            Unit::Millimetres(mm) => (mm * self.dpmm as f32) as i32,
        }
    }

    pub async fn render(&self) -> anyhow::Result<command::CommandSequence> {
        let mut output = CommandSequence(vec![]);

        for c in &self.content {
            match c {
                LabelContent::Image { img, x, y, w, h } => {
                    let img = img.resize_to_fill(
                        self.unit_to_dots(w),
                        self.unit_to_dots(h),
                        ::image::imageops::FilterType::Lanczos3,
                    );

                    let img_serialized =
                        crate::util::image::SerializedImage::new_z64(&img);

                    output.push(ZplCommand::MoveOrigin(
                        self.unit_to_dots(x),
                        self.unit_to_dots(y),
                    ));
                    output.push(ZplCommand::RenderImage(img_serialized));
                }
                LabelContent::Svg { code, x, y, w, h } => {
                    let img_serialized =
                        crate::util::image::SerializedImage::from_svg(
                            code.to_string(),
                            self.unit_to_dots(w),
                            self.unit_to_dots(h),
                        )
                        .context("Could not load SVG")?;

                    output.push(ZplCommand::MoveOrigin(
                        self.unit_to_dots(x),
                        self.unit_to_dots(y),
                    ));
                    output.push(ZplCommand::RenderImage(img_serialized));
                }
                LabelContent::SvgTree { tree, x, y, w, h } => {
                    let img_serialized =
                        crate::util::image::SerializedImage::from_svg_tree(
                            tree.clone(),
                            self.unit_to_dots(w),
                            self.unit_to_dots(h),
                        )
                        .context("Could not load SVG")?;

                    output.push(ZplCommand::MoveOrigin(
                        self.unit_to_dots(x),
                        self.unit_to_dots(y),
                    ));
                    output.push(ZplCommand::RenderImage(img_serialized));
                }
                LabelContent::QrCode {
                    content,
                    x,
                    y,
                    zoom,
                } => {
                    output.push(ZplCommand::MoveOrigin(
                        self.unit_to_dots(x),
                        self.unit_to_dots(y),
                    ));
                    output.push(ZplCommand::FieldModeQRCode { zoom: *zoom });
                    output.push(ZplCommand::FieldData(format!(
                        "{}A,{}",
                        "Q", // Error correction level
                        content
                    )));
                }
            }
        }

        Ok(output)
    }

    pub async fn print(
        &self,
        options: &PrintOptions,
    ) -> anyhow::Result<CommandSequence> {
        let mut commands = make_preamble();
        let copies = options.copies;

        commands.append(CommandSequence(vec![
            ZplCommand::StartLabel,
            ZplCommand::SetPostPrintAction(PostPrintAction::Cut),
            ZplCommand::SetPrintWidth(self.width * self.dpmm),
            ZplCommand::SetLabelLength(self.height * self.dpmm),
            ZplCommand::SetHorizontalShift(0),
        ]));

        if let Some(calib) = &options.calibration {
            let home_x = self.signed_unit_to_dots(&calib.home_x);
            commands.append(CommandSequence(vec![
                ZplCommand::SetVerticalShift(home_x),
            ]));
        }

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
        ZplCommand::StartLabel,
        ZplCommand::SetEncoding(28),
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
