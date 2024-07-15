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
    Raw(String),
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
    RenderQRCode {
        content: String,
        zoom: u32,
    },
    Start,
    End,
}

impl From<ZplCommand> for String {
    fn from(value: ZplCommand) -> Self {
        match value {
            ZplCommand::Raw(s) => s,
            ZplCommand::Magic => vec!["CT~~CD,~CC^~CT~", "^XA~TA000~JSN^LT0^MNW"].join("\n"),
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
        }
    }
}

#[test]
fn test_raw() {
    let c = ZplCommand::Raw("Abc".to_string());
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
