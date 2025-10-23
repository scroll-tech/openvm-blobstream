use crate::da_oracle::{DataCommitmentStored, SP1BlobstreamInstance};
use crate::verifier::{
    AttestationProof, BinaryMerkleProof, DataRootTuple, Namespace, NamespaceMerkleMultiproof,
    NamespaceNode, SharesProof,
};
use alloy_primitives::{B256, Bytes, U256, hex};
use alloy_provider::Provider;
use serde::Deserialize;
use tendermint::serializers;

#[derive(Debug, Deserialize)]
pub struct GetTx {
    #[serde(with = "serializers::from_str")]
    pub height: u64,
    pub proof: TxProof,
}

#[derive(Debug, Deserialize)]
pub struct MerkleProof {
    #[serde(with = "serializers::from_str")]
    pub total: u64,
    #[serde(with = "serializers::from_str")]
    pub index: u64,
    #[serde(with = "serializers::bytes::vec_base64string")]
    pub aunts: Vec<Vec<u8>>,
}

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

impl SharesProof {
    pub fn new(
        block_height: u64,
        celestia_block_data_hash: B256,
        inclusion_proof_nonce: U256,
        inclusion_proof: MerkleProof,
        tx_proof: TxProof,
    ) -> Self {
        SharesProof {
            data: tx_proof
                .data
                .into_iter()
                .map(|data| Bytes::from(data))
                .collect(),

            shareProofs: tx_proof
                .share_proofs
                .into_iter()
                .map(|proof| NamespaceMerkleMultiproof {
                    beginKey: U256::from(proof.start),
                    endKey: U256::from(proof.end),
                    sideNodes: proof
                        .nodes
                        .into_iter()
                        .map(|node| NamespaceNode::from(node.as_ref()))
                        .collect(),
                })
                .collect(),

            namespace: Namespace::new(tx_proof.namespace_version, tx_proof.namespace_id.as_ref()),

            rowRoots: tx_proof
                .row_proof
                .row_roots
                .iter()
                .map(|root| NamespaceNode::from(root.as_ref()))
                .collect(),

            rowProofs: tx_proof
                .row_proof
                .proofs
                .into_iter()
                .map(BinaryMerkleProof::from)
                .collect(),

            attestationProof: AttestationProof {
                tupleRootNonce: inclusion_proof_nonce,

                tuple: DataRootTuple {
                    height: U256::from(block_height),
                    dataRoot: celestia_block_data_hash,
                },

                proof: inclusion_proof.into(),
            },
        }
    }
}

impl From<MerkleProof> for BinaryMerkleProof {
    fn from(proof: MerkleProof) -> Self {
        Self {
            sideNodes: proof
                .aunts
                .iter()
                .map(|node| B256::from_slice(node.as_ref()))
                .collect(),
            key: U256::from(proof.index),
            numLeaves: U256::from(proof.total),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: T,
}

#[derive(Debug, Deserialize)]
struct GetDataRootInclusionProof {
    proof: MerkleProof,
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
}

pub async fn get_celestia_tx_by_hash(rpc_url: &str, tx_hash: &[u8]) -> eyre::Result<GetTx> {
    let tx = reqwest::get(format!(
        "{rpc_url}/tx?hash={}&prove=true",
        hex::encode_prefixed(tx_hash),
    ))
    .await?
    .json::<RpcResponse<GetTx>>()
    .await?
    .result;
    Ok(tx)
}

pub async fn get_celestia_data_root_inclusion_proof(
    rpc_url: &str,
    height: u64,
    start: u64,
    end: u64,
) -> eyre::Result<MerkleProof> {
    let proof = reqwest::get(format!(
        "{rpc_url}/data_root_inclusion_proof?height={height}&start={start}&end={end}",
    ))
    .await?
    .json::<RpcResponse<GetDataRootInclusionProof>>()
    .await?
    .result
    .proof;
    Ok(proof)
}

pub async fn find_commit_tx(
    blob_stream_contract: &SP1BlobstreamInstance<impl Provider>,
    ethereum_client: &impl Provider,
    tx_height: u64,
) -> eyre::Result<(B256, DataCommitmentStored)> {
    let latest_eth_block_number = ethereum_client.get_block_number().await?;
    let filter = blob_stream_contract
        .DataCommitmentStored_filter()
        .from_block(latest_eth_block_number - 50000)
        .to_block(latest_eth_block_number);
    let logs = filter.query().await?;
    let mut tx_hash = Option::<B256>::None;
    let mut event = Option::<DataCommitmentStored>::None;

    for (e, log) in logs.into_iter() {
        if e.startBlock <= tx_height && e.endBlock > tx_height {
            tx_hash = Some(log.transaction_hash.unwrap());
            event = Some(e);
            break;
        }
    }
    Ok((tx_hash.unwrap(), event.unwrap()))
}

mod vec_hexstring {
    use alloy_primitives::hex;
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
}
