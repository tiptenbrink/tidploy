use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct SecretOutput {
    pub(crate) key: String,
    pub(crate) value: String,
}
