#[derive(Clone)]
pub enum ZplCommand {
    Raw(String),
}

impl From<ZplCommand> for String {
    fn from(value: ZplCommand) -> Self {
        match value {
            ZplCommand::Raw(s) => s,
        }
    }
}

#[test]
fn test_raw() {
    let c = ZplCommand::Raw("Abc".to_string());
    assert_eq!(String::from(c), "Abc");
}
