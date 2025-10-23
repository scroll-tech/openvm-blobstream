pub use Verifier::*;
use alloy_primitives::B256;
use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};

sol!(
    #[derive(Debug, Serialize, Deserialize)]
    Verifier,
    "src/Verifier.json"
);

impl Namespace {
    pub fn new(version: u8, id: &[u8]) -> Self {
        let id: [u8; 28] = id.try_into().expect("invalid namespace id length");
        Self {
            version: version.into(),
            id: id.into(),
        }
    }
}

impl From<&[u8]> for Namespace {
    fn from(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 29);
        let version = bytes[0];
        let mut id = [0u8; 28];
        id.copy_from_slice(&bytes[1..29]);
        Self {
            version: version.into(),
            id: id.into(),
        }
    }
}

impl From<&[u8]> for NamespaceNode {
    fn from(bytes: &[u8]) -> Self {
        const LEN: usize = 29 * 2 + 32;
        let bytes: &[u8; LEN] = bytes.try_into().expect("invalid length");
        let min = Namespace::from(&bytes[0..29]);
        let max = Namespace::from(&bytes[29..58]);
        let digest = B256::from_slice(&bytes[58..90]);
        NamespaceNode { min, max, digest }
    }
}
