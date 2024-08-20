use crate::image::{SerializedImage};

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
    HostIndication,
    HostRamStatus,
    HostStatusReturn,
    Start,
    End,
}

#[derive(Default, Debug)]
pub struct DeviceInfo {
    pub string1: DeviceInfo1,
    pub string2: DeviceInfo2,
    pub string3: DeviceInfo3,
    pub indication: DeviceInfoIndication,
    pub ram: DeviceInfoRam,
}

#[derive(Default, Debug)]
pub struct DeviceInfo1 {
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
pub struct DeviceInfo2 {
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
pub struct DeviceInfo3 {
    pub x_password: String,
    pub y_static_ram: bool,
}

#[derive(Default, Debug)]
pub struct DeviceInfoRam {
    pub total: u32,
    pub maximum_to_user: u64,
    pub available_to_user: u64,
}

#[derive(Default, Debug)]
pub struct DeviceInfoIndication {
    pub model: String,
    pub version: String,
    pub dpmm: u32,
    /// FIXME: a more specific type?
    pub memory: String,
}

impl ZplCommand {
    /// How many lines of text
    pub fn how_many_lines_of_text(&self) -> u32 {
        match self {
            ZplCommand::HostIndication => 1,
            ZplCommand::HostRamStatus => 1,
            ZplCommand::HostStatusReturn => 3,
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
            ZplCommand::HostIndication => "~HI".to_string(),
            ZplCommand::HostRamStatus => "~HM".to_string(),
            ZplCommand::HostStatusReturn => "~HS".to_string(),
        }
    }
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
