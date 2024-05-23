use crate::command::ZplCommand;

#[derive(Clone)]
pub struct Label {
    pub commands: Vec<ZplCommand>,
}

impl From<Label> for String {
    fn from(value: Label) -> Self {
        value
            .commands
            .into_iter()
            .map(|c| String::from(c))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

#[test]
fn test_basic() {
    let l = Label {
        commands: vec![
            ZplCommand::Raw("abc".to_string()),
            ZplCommand::Raw("def".to_string()),
        ],
    };
    assert_eq!(String::from(l), "abc\ndef".to_string());
}
