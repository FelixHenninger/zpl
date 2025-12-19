use std::sync::Arc;

use base64::{engine::GeneralPurpose, Engine as _};
use serde::de::{Deserialize, Error, Unexpected};
use serde::ser::Serialize;

/// A serde compatible decoding of a data URI.
///
/// Used to pass arbitrary binary data in a Json/String compatible encoding with web-native tools.
pub struct DataUri {
    pub mime: String,
    pub data: Arc<[u8]>,
}

impl<'lt> Deserialize<'lt> for DataUri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'lt>,
    {
        let inner: &str = <_>::deserialize(deserializer)?;

        if !inner.starts_with("data:") {
            let unexpected = Unexpected::Str(inner);
            return Err(D::Error::invalid_value(unexpected, &"a data URI"));
        }

        let Some(sep) = inner.find(',') else {
            let unexpected = Unexpected::Str(inner);
            return Err(D::Error::invalid_value(
                unexpected,
                &"a separator `,` indicating the start of data",
            ));
        };

        let mut mime_part = &inner[5..sep];
        let data_part = &inner[sep + 1..];
        let is_base64;

        if let Some((mime, options)) = mime_part.split_once(';') {
            mime_part = mime;
            is_base64 = options.contains("base64");
        } else {
            is_base64 = false;
        };

        let mime = mime_part.to_owned();
        let data = if is_base64 {
            match GeneralPurpose::new(
                &base64::alphabet::STANDARD,
                base64::engine::GeneralPurposeConfig::default()
                    .with_decode_padding_mode(
                        base64::engine::DecodePaddingMode::Indifferent,
                    ),
            )
            .decode(data_part)
            {
                Ok(data) => Arc::<[u8]>::from(data),
                Err(b64err) => {
                    let unexpected = Unexpected::Str(data_part);
                    log::warn!("Invalid client Base 64 {b64err}");
                    return Err(D::Error::invalid_value(
                        unexpected,
                        &"a base64 encoded string",
                    ));
                }
            }
        } else {
            Arc::<[u8]>::from(data_part.as_bytes())
        };

        Ok(DataUri { mime, data })
    }
}

impl Serialize for DataUri {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let engine = GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::GeneralPurposeConfig::default(),
        );

        let data = base64::display::Base64Display::new(&self.data, &engine);
        let value = format!("data:{};base64,{}", self.mime, data);
        serializer.serialize_str(&value)
    }
}

#[test]
fn data_uri_decoding() {
    let uri: DataUri =
        serde_json::from_str("\"data:image/svg,<svg></svg>\"").unwrap();
    assert_eq!(uri.mime, "image/svg");
    assert_eq!(*uri.data, *b"<svg></svg>");

    let uri: DataUri =
        serde_json::from_str("\"data:application/png;base64,AEAQ\"").unwrap();
    assert_eq!(uri.mime, "application/png");
    assert_eq!(*uri.data, *b"\x00\x40\x10");
}
