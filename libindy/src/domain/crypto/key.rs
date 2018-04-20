extern crate indy_crypto;

use self::indy_crypto::utils::json::{JsonDecodable, JsonEncodable};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Key {
    pub verkey: String,
    pub signkey: String
}

impl Key {
    pub fn new(verkey: String, signkey: String) -> Key {
        Key {
            verkey,
            signkey
        }
    }
}

impl JsonEncodable for Key {}

impl<'a> JsonDecodable<'a> for Key {}

#[derive(Serialize, Deserialize)]
pub struct KeyInfo {
    pub seed: Option<String>,
    pub crypto_type: Option<String>
}

impl JsonEncodable for KeyInfo {}

impl<'a> JsonDecodable<'a> for KeyInfo {}

