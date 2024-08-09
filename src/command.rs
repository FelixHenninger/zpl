use crate::image::SerializedImage;

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
pub enum ZplCommand {
    Raw {
        text: String,
        /// How many 'lines' / fields of text to read back after the sequence, to purge what the
        /// print would send from the buffer. If we want to raw-ify commands that send non-text
        /// delimited fields then we should look into having a proper sequence encoded in this
        /// field.
        response_lines: u32,
    },
    Magic,
    PersistConfig,
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
    LabelSetup {
        w: u32,
        h: u32,
        dpmm: u32,
    },
    SetPostPrintAction(PostPrintAction),
    SetHorizontalShift(usize),
    SetVerticalShift(isize),
    SetTearOffPosition(isize),
    MoveOrigin(u32, u32),
    PrintQuantity {
        total: u32,
        pause_and_cut_after: u32,
        replicates: u32,
        cut_only: bool,
    },
    Image(crate::image::SerializedImage),
    RenderQRCode {
        content: String,
        zoom: u32,
    },
    RequestHostIdentification,
    RequestHostRamStatus,
    RequestHostStatus,
    Start,
    End,
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
            ZplCommand::Raw { text, .. } => text,
            ZplCommand::Magic => ["CT~~CD,~CC^~CT~", "^XA~TA000~JSN^LT0^MNW"].join("\n"),
            // Removed:
            // -
            // - ^PON -> rotate by 180 degrees
            // - ^PMN -> decommission?
            ZplCommand::PersistConfig => "^JUS".to_string(),
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
            ZplCommand::LabelSetup { w, h, dpmm } => {
                format!("^PW{:0>3}\n^LL{:0>4}", w * dpmm, h * dpmm)
            }
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
            ZplCommand::MoveOrigin(x, y) => format!("^FO{},{}", x, y),
            ZplCommand::PrintQuantity {
                total,
                pause_and_cut_after,
                replicates,
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
            ZplCommand::Image(SerializedImage {
                byte_count,
                total_field_count,
                bytes_per_row,
                data,
            }) => format!("^GFA,{byte_count},{total_field_count},{bytes_per_row},{data}^FS"),
            ZplCommand::RenderQRCode { content, zoom } => {
                let config = format!(
                    "^BQ{},{},{},{},{}",
                    "N",  // Orientation
                    2,    // Model
                    zoom, // Magnification (1-100)
                    "Q",  // Error correction
                    7     // Mask
                );
                let data = format!(
                    "^FD{}A,{}",
                    "Q", // Error correction level
                    content
                );
                format!("{config}\n{data}")
            }
            ZplCommand::Start => "^XA".to_string(),
            ZplCommand::End => "^XZ".to_string(),
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
        text: "Abc".to_string(),
        response_lines: 0,
    };

    assert_eq!(String::from(c), "Abc");
}

#[test]
fn test_setup() {
    let c = ZplCommand::LabelSetup {
        w: 57,
        h: 32,
        dpmm: 12,
    };
    assert_eq!(String::from(c), "^PW684\n^LL0384");
}

pub struct CommandSequence(pub Vec<ZplCommand>);

impl CommandSequence {
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
