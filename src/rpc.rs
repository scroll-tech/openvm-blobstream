use alloy::hex;
use alloy::primitives::{Address, address};
use serde::Deserialize;
use tendermint::serializers;

pub const CELESTIA_RPC_URL: &str = "https://celestia-rpc.publicnode.com:443";
pub const ETHEREUM_RPC_URL: &str = "https://ethereum-rpc.publicnode.com";
pub const BLOBSTREAM_CONTRACT_ADDRESS: Address =
    address!("0x7Cf3876F681Dbb6EdA8f6FfC45D66B996Df08fAe");

#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: T,
}

#[derive(Debug, Deserialize)]
pub struct GetTx {
    // #[serde(with = "serializers::bytes::hexstring")]
    // pub hash: Vec<u8>,
    #[serde(with = "serializers::from_str")]
    pub height: u64,
    // pub index: u64,
    // pub tx_result: TxResult,
    // pub tx: String,
    pub proof: TxProof,
}

#[derive(Debug, Deserialize)]
struct GetDataRootInclusionProof {
    proof: MerkleProof,
}

// #[derive(Debug, Deserialize)]
// pub struct TxResult {
//     // code: u64,
//     #[serde(with = "serializers::bytes::base64string")]
//     pub data: Vec<u8>,
//     // log: String,
//     // info: String,
//     // gas_wanted: String,
//     // gas_used: String,
//     // events: Vec<Struct>,
//     // codespace: String,
// }

#[derive(Debug, Deserialize)]
pub struct TxProof {
    #[serde(with = "serializers::bytes::vec_base64string")]
    pub data: Vec<Vec<u8>>,
    pub share_proofs: Vec<ShareProof>,
    #[serde(with = "serializers::bytes::base64string")]
    pub namespace_id: Vec<u8>,
    pub row_proof: RowProof,
    pub namespace_version: u8,
}

#[derive(Debug, Deserialize)]
pub struct ShareProof {
    #[serde(default)]
    pub start: u64,
    pub end: u64,
    #[serde(with = "serializers::bytes::vec_base64string")]
    pub nodes: Vec<Vec<u8>>,
}

#[derive(Debug, Deserialize)]
pub struct RowProof {
    #[serde(with = "vec_hexstring")]
    pub row_roots: Vec<Vec<u8>>,
    pub proofs: Vec<MerkleProof>,
    // pub start_row: u64,
    // pub end_row: u64,
}

#[derive(Debug, Deserialize)]
pub struct MerkleProof {
    #[serde(with = "serializers::from_str")]
    pub total: u64,
    #[serde(with = "serializers::from_str")]
    pub index: u64,
    // #[serde(with = "serializers::bytes::base64string")]
    // pub leaf_hash: Vec<u8>,
    #[serde(with = "serializers::bytes::vec_base64string")]
    pub aunts: Vec<Vec<u8>>,
}

// #[derive(Deserialize)]
// struct Struct1 {
//     pub key: String,
//     pub value: String,
//     pub index: bool,
// }

// #[derive(Serialize, Deserialize)]
// struct Struct {
//     #[serde(rename = "type")]
//     pub r#type: String,
//     pub attributes: Vec<Struct1>,
// }

pub async fn get_tx(tx_hash: &[u8]) -> eyre::Result<GetTx> {
    let tx = reqwest::get(format!(
        "{CELESTIA_RPC_URL}/tx?hash={}&prove=true",
        hex::encode_prefixed(tx_hash),
    ))
    .await?
    .json::<RpcResponse<GetTx>>()
    .await?
    .result;
    Ok(tx)
}

pub async fn get_data_root_inclusion_proof(
    height: u64,
    start: u64,
    end: u64,
) -> eyre::Result<MerkleProof> {
    let proof = reqwest::get(format!(
        "{CELESTIA_RPC_URL}/data_root_inclusion_proof?height={height}&start={start}&end={end}",
    ))
    .await?
    .json::<RpcResponse<GetDataRootInclusionProof>>()
    .await?
    .result
    .proof;
    Ok(proof)
}

mod vec_hexstring {
    use alloy::hex;
    use serde::{Deserialize, Deserializer};

    /// Deserialize array into `Vec<Vec<u8>>`
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<Vec<String>>::deserialize(deserializer)?
            .unwrap_or_default()
            .into_iter()
            .map(|s| hex::decode(s).map_err(serde::de::Error::custom))
            .collect()
    }

    // /// Serialize from `Vec<T>` into `Vec<base64string>`
    // pub fn serialize<S, T>(value: &[T], serializer: S) -> Result<S::Ok, S::Error>
    // where
    //     S: Serializer,
    //     T: AsRef<[u8]>,
    // {
    //     let hex_strings = value
    //         .iter()
    //         .map(|v| hex::encode(v.as_ref()))
    //         .collect::<Vec<String>>();
    //     serializer.collect_seq(hex_strings)
    // }
}
