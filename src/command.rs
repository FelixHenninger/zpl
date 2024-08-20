use crate::util::image::SerializedImage;

#[derive(Clone)]
pub enum PostPrintAction {
    /// Present only, let user tear off.
    TearOff,
    /// Present and cut.
    Cut,
}

#[derive(Clone)]
pub enum MediaType {
    /// Color is in the label, turning dark on heating.
    Direct,
    /// Color in a separate color strip.
    Transfer,
}

#[derive(Clone)]
pub enum BackfeedSequence {
    /// 100 percent backfeed after printing and cutting
    AfterPrinting,
    /// 0 percent backfeed after printing and cutting,
    /// followed by 100 percent before printing the next label
    BeforePrinting,
    /// Default — 90 percent backfeed after printing
    Default,
    /// No backfeed
    Off,
    /// Percentage value, 10 to 90%
    /// The value entered must be a multiple of 10. Values not divisible by 10
    /// are rounded to the nearest acceptable value. For example, 55 is accepted
    /// as 50 percent backfeed.
    Percent(u8),
}

#[derive(Clone)]
pub enum MediaTracking {
    /// Continuous media
    ///
    /// Expects media label size to be setup independently, as no physical
    /// characteristic indicates label seperation (e.g. through SetLabelSize).
    Continuous,
    /// Continuous media, variable length
    ///
    /// Extends label size to label content; expects initial label size
    /// to be setup independently (e.g. through SetLabelSize)
    ContinuousVariableLength,
    /// Non-continuous media web (gap) sensing
    NonContinuousWebSensing,
    /// Non-continuous media mark sensing
    ///
    /// Expects a mark on the media, with a constant offset relative to
    /// the point of separation (perforation, cut point, etc.). The offset
    /// is supplied as a vertical distance in dots. By default, it is zero,
    /// meaning that the mark indicates the exact point of separation.
    NonContinuousMarked(i16),
    /// Auto-detect the media type during calibration
    Autodetect,
}

#[derive(Clone)]
pub enum ZplCommand {
    Raw {
        command: String,
        /// How many 'lines' / fields of text to read back after the sequence,
        /// to purge what the printer would send from the buffer. If we want to
        /// supply raw commands that send non-text delimited fields then we
        /// should look into having a proper sequence encoded in this field.
        response_lines: u32,
    },
    StartLabel,
    EndLabel,
    PersistConfiguration,
    /// Change the delimiter character that separates parameter values
    /// for ZPL commands. Defaults to comma.
    SetDelimiter(char),
    /// Change the control command prefix. Defaults to tilde.
    SetControlCommandPrefix(char),
    /// Change the format command prefix. Defaults to caret.
    SetFormatCommandPrefix(char),
    /// Change the point at which the label stock is pulled back
    /// in order to print the next label (i.e. after the last or before the
    /// next label)
    SetBackfeedSequence(BackfeedSequence),
    /// Set the media type and how to detect label separation
    SetMediaTracking(MediaTracking),
    /// Set print intensity
    SetDarkness(usize),
    SetEncoding(usize),
    SetInverted(bool),
    SetHalfDensity(bool),
    SetHome(u32, u32),
    SetSpeed {
        print: usize,
        slew: usize,
    },
    SetMediaType(MediaType),
    SetPrintWidth(u32),
    SetLabelLength(u32),
    SetPostPrintAction(PostPrintAction),
    SetHorizontalShift(usize),
    /// Move the entire label content vertically up or down, relative
    /// to the upper edge of the label. Accepts an offset of up to 120
    /// dots.
    SetVerticalShift(isize),
    SetTearOffPosition(isize),
    /// Mirror the label vertically
    SetMirrored(bool),
    /// Rotate the label by 180 degrees
    SetFlipped(bool),
    MoveOrigin(u32, u32),
    PrintQuantity {
        total: u32,
        pause_and_cut_after: u32,
        replicates_per_serial: u32,
        cut_only: bool,
    },
    RenderImage(crate::util::image::SerializedImage),
    FieldOrigin(u32, u32),
    FieldData(String),
    FieldModeQRCode {
        zoom: u32,
    },
    RequestHostIdentification,
    RequestHostRamStatus,
    RequestHostStatus,
}

#[derive(Default, Debug)]
pub struct HostStatus {
    pub string1: HostStatus1,
    pub string2: HostStatus2,
    pub string3: HostStatus3,
    pub identification: HostIdentification,
    pub ram_status: HostRamStatus,
}

#[derive(Default, Debug)]
pub struct HostStatus1 {
    pub a_communication: u32,
    pub b_paper_out: bool,
    pub c_pause: bool,
    pub d_label_length: u32,
    pub e_number_formats: u32,
    pub f_buffer_full: bool,
    pub g_communication_diagnostics: bool,
    pub h_partial_format: bool,
    pub j_corrupt_ram: bool,
    pub k_temperature_low: bool,
    pub l_temperature_high: bool,
}

#[derive(Default, Debug)]
pub struct HostStatus2 {
    pub m_settings: u8,
    pub o_head_up: bool,
    pub p_ribbon_out: bool,
    pub q_thermal_transfer_mode: bool,
    pub r_print_mode: u32,
    pub s_print_width_mode: u8,
    pub t_label_waiting: bool,
    pub u_labels_remaining: u32,
    pub v_format_printing: bool,
    pub w_number_graphics_stored: u32,
}

#[derive(Default, Debug)]
pub struct HostStatus3 {
    pub x_password: String,
    pub y_static_ram: bool,
}

#[derive(Default, Debug)]
pub struct HostRamStatus {
    pub total: u32,
    pub maximum_to_user: u64,
    pub available_to_user: u64,
}

#[derive(Default, Debug)]
pub struct HostIdentification {
    pub model: String,
    pub version: String,
    pub dpmm: u32,
    /// FIXME: a more specific type?
    pub memory: String,
}

impl ZplCommand {
    /// How many lines of data to expect in response to a command
    pub fn expected_response_lines(&self) -> u32 {
        match self {
            ZplCommand::RequestHostIdentification => 1,
            ZplCommand::RequestHostRamStatus => 1,
            ZplCommand::RequestHostStatus => 3,
            ZplCommand::Raw { response_lines, .. } => *response_lines,
            _ => 0,
        }
    }
}

impl From<ZplCommand> for String {
    fn from(value: ZplCommand) -> Self {
        match value {
            ZplCommand::Raw { command: text, .. } => text,
            // Removed:
            // -
            // - ^PON -> rotate by 180 degrees
            ZplCommand::StartLabel => "^XA".to_string(),
            ZplCommand::EndLabel => "^XZ".to_string(),
            ZplCommand::PersistConfiguration => "^JUS".to_string(),
            ZplCommand::SetDelimiter(delimiter) => format!("~CD{delimiter}"),
            ZplCommand::SetControlCommandPrefix(prefix) => format!("~CT{prefix}"),
            ZplCommand::SetFormatCommandPrefix(prefix) => format!("~CC{prefix}"),
            ZplCommand::SetBackfeedSequence(sequence) => {
                let value = match sequence {
                    BackfeedSequence::AfterPrinting => "A".to_string(),
                    BackfeedSequence::BeforePrinting => "B".to_string(),
                    BackfeedSequence::Default => "N".to_string(),
                    BackfeedSequence::Off => "O".to_string(),
                    BackfeedSequence::Percent(p) => format!("{p}")
                };

                format!("~JS{value}")
            },
            ZplCommand::SetMediaTracking(tracking) => {
                let t = match tracking {
                    MediaTracking::Continuous => "N".to_string(),
                    MediaTracking::ContinuousVariableLength => "V".to_string(),
                    MediaTracking::NonContinuousWebSensing => "W".to_string(),
                    MediaTracking::NonContinuousMarked(offset) => format!("M,{offset}"),
                    MediaTracking::Autodetect => "A".to_string(),
                };

                format!("^MN{t}")
            }
            ZplCommand::SetHalfDensity(d) => format!("^JM{}", if d { "B" } else { "A" }),
            ZplCommand::SetDarkness(e) => format!("~SD{}", e),
            ZplCommand::SetEncoding(e) => format!("^CI{}", e),
            ZplCommand::SetHome(x, y) => format!("^LH{},{}", x, y),
            ZplCommand::SetInverted(i) => {
                format!("^LR{}", if i { "Y" } else { "N" })
            }
            ZplCommand::SetMediaType(t) => {
                let t = match t {
                    MediaType::Direct => "D",
                    MediaType::Transfer => "T",
                };
                format!("^MT{}", t)
            }
            ZplCommand::SetSpeed { print, slew } => format!("^PR{},{}", print, slew),
            ZplCommand::SetPrintWidth(w) => format!("^PW{:0>3}", w),
            ZplCommand::SetLabelLength(l) => format!("^LL{:0>4}", l),
            ZplCommand::SetPostPrintAction(a) => {
                let c = match a {
                    PostPrintAction::TearOff => "T",
                    PostPrintAction::Cut => "C",
                };

                format!("^MM{}", c)
            }
            ZplCommand::SetHorizontalShift(s) => format!("^LS{}", s),
            ZplCommand::SetVerticalShift(s) => format!("^LT{}", s),
            ZplCommand::SetTearOffPosition(p) => format!("~TA{:>+04}", p),
            ZplCommand::SetMirrored(enabled) => {
                let mirrored = match enabled {
                    true => "Y",
                    false => "N",
                };

                format!("^PM{}", mirrored)
            },
            ZplCommand::SetFlipped(enabled) => {
                let flipped = match enabled {
                    true => "I",
                    false => "N",
                };

                format!("^PO{}", flipped)
            },
            ZplCommand::MoveOrigin(x, y) => format!("^FO{},{}", x, y),
            ZplCommand::PrintQuantity {
                total,
                pause_and_cut_after,
                replicates_per_serial: replicates,
                cut_only,
            } => {
                format!(
                    "^PQ{},{},{},{}",
                    total,
                    pause_and_cut_after,
                    replicates,
                    if cut_only { "Y" } else { "N" }
                )
            }
            ZplCommand::RenderImage(SerializedImage {
                byte_count,
                total_field_count,
                bytes_per_row,
                data,
            }) => format!("^GFA,{byte_count},{total_field_count},{bytes_per_row},{data}^FS"),
            ZplCommand::FieldOrigin(x, y) => format!("^FO{x},{y}"),
            ZplCommand::FieldData(data) => format!("^FD{data}"),
            ZplCommand::FieldModeQRCode { zoom } => {
                format!(
                    "^BQ{},{},{},{},{}",
                    "N",  // Orientation
                    2,    // Model
                    zoom, // Magnification (1-100)
                    "Q",  // Error correction
                    7     // Mask
                )
            }
            ZplCommand::RequestHostIdentification => "~HI".to_string(),
            ZplCommand::RequestHostRamStatus => "~HM".to_string(),
            ZplCommand::RequestHostStatus => "~HS".to_string(),
        }
    }
}

pub fn total_expected_response_lines(commands: &[ZplCommand]) -> u32 {
    commands
        .iter()
        .map(ZplCommand::expected_response_lines)
        .sum()
}

#[test]
fn test_raw() {
    let c = ZplCommand::Raw {
        command: "Abc".to_string(),
        response_lines: 0,
    };

    assert_eq!(String::from(c), "Abc");
}

#[test]
fn test_setup() {
    let c = CommandSequence(vec![
        ZplCommand::SetPrintWidth(684),
        ZplCommand::SetLabelLength(384),
    ]);
    assert_eq!(String::from(c), "^PW684\n^LL0384");
}

pub struct CommandSequence(pub Vec<ZplCommand>);

impl CommandSequence {
    pub fn append(&mut self, mut c: Self) {
        self.0.append(&mut c.0)
    }

    pub fn push(&mut self, c: ZplCommand) {
        self.0.push(c)
    }

    pub fn expected_response_lines(&self) -> u32 {
        total_expected_response_lines(&self.0)
    }
}

impl core::fmt::Display for CommandSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for inner in &self.0 {
            writeln!(f, "{}", String::from(inner.clone()))?;
        }

        Ok(())
    }
}

impl From<CommandSequence> for String {
    fn from(sequence: CommandSequence) -> Self {
        let commands = sequence.0;

        commands
            .into_iter()
            .map(String::from)
            .collect::<Vec<String>>()
            .join("\n")
    }
}
