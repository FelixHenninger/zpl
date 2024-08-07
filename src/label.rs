use crate::command::ZplCommand;

#[derive(Clone)]
pub struct Label {
    pub commands: Vec<ZplCommand>,
}

impl Label {
    pub fn expected_response_lines(&self) -> u32 {
        self.commands
            .iter()
            .map(ZplCommand::expected_response_lines)
            .sum()
    }
}

impl core::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for inner in &self.commands {
            writeln!(f, "{}", String::from(inner.clone()))?;
        }

        Ok(())
    }
}

impl From<Label> for String {
    fn from(value: Label) -> Self {
        value
            .commands
            .into_iter()
            .map(String::from)
            .collect::<Vec<String>>()
            .join("\n")
    }
}

#[test]
fn test_basic() {
    let l = Label {
        commands: vec![
            ZplCommand::Raw {
                text: "abc".to_string(),
                response_lines: 0,
            },
            ZplCommand::Raw {
                text: "def".to_string(),
                response_lines: 0,
            },
        ],
    };
    assert_eq!(String::from(l), "abc\ndef".to_string());
}
