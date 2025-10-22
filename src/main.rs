use crate::blobstream::{SP1Blobstream, find_commit_tx};
use crate::guest::{verify_celestia_block_included, verify_shares_to_data_root_tuple_root};
use crate::rpc::{
    BLOBSTREAM_CONTRACT_ADDRESS, CELESTIA_RPC_URL, ETHEREUM_RPC_URL, get_data_root_inclusion_proof,
};
use crate::verifier::{
    AttestationProof, BinaryMerkleProof, DataRootTuple, Namespace, NamespaceMerkleMultiproof,
    NamespaceNode, SharesProof,
};
use alloy::consensus::Transaction;
use alloy::hex;
use alloy::primitives::{B256, Bytes, U256};
use alloy::providers::Provider;
use tendermint_rpc::Client;
use tracing::*;
use tracing_subscriber::EnvFilter;

const DATA_COMMITMENT_MAX: u64 = 10000;

const TX_HASH: &[u8] = &hex!("38D01D9A80A1FB6D6550E0B8C6487AF229A0F6741B6AD84E7B2208819E80214C");

/// This won't be part of final library, just for demo purpose.
mod rpc;

mod da_oracle {
    alloy::sol!(
        #[sol(rpc)]
        IDAOracle,
        "verifier/out/IDAOracle.sol/IDAOracle.json"
    );
    pub use IDAOracle::*;
}

mod blobstream;
mod verifier {
    alloy::sol!(Verifier, "verifier/out/Verifier.sol/Verifier.json");

    pub use Verifier::*;
    use alloy::primitives::B256;

    impl Namespace {
        pub fn new(version: u8, id: &[u8]) -> Self {
            let id: [u8; 28] = id.try_into().expect("invalid namespace id length");
            Namespace {
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
            Namespace {
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
}

mod guest;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let ethereum_client =
        alloy::providers::ProviderBuilder::new().connect_http(ETHEREUM_RPC_URL.parse()?);
    let celestia_client = tendermint_rpc::HttpClient::new(CELESTIA_RPC_URL)?;
    let blobstream_contract =
        SP1Blobstream::new(BLOBSTREAM_CONTRACT_ADDRESS, ethereum_client.clone());

    let blobstream_vk = blobstream_contract.blobstreamProgramVkey().call().await?;
    let blobstream_vk = hex::encode_prefixed(blobstream_vk);
    info!("Blobstream VK hash: {blobstream_vk}");

    let data_commitment_max = blobstream_contract.DATA_COMMITMENT_MAX().call().await?;
    assert_eq!(data_commitment_max, DATA_COMMITMENT_MAX);

    let tx = rpc::get_tx(TX_HASH).await?;
    let pfb_height = tx.height;
    info!("PayForBlobs tx at celestia height #{pfb_height}");
    let celestia_block = celestia_client.block(pfb_height as u32).await?.block;
    let celestia_block_data_hash =
        B256::from_slice(celestia_block.header.data_hash.unwrap().as_ref());

    let (commit_tx_hash, event) =
        find_commit_tx(&blobstream_contract, &ethereum_client, pfb_height).await?;
    info!("found DataCommitmentStored event in ethereum tx {commit_tx_hash}: {event:?}");

    let inclusion_proof =
        get_data_root_inclusion_proof(pfb_height, event.startBlock, event.endBlock).await?;

    // sanity check: verify attestation proof on-chain
    let on_chain_verifier =
        da_oracle::IDAOracle::new(BLOBSTREAM_CONTRACT_ADDRESS, ethereum_client.clone());
    let result = on_chain_verifier
        .verifyAttestation(
            event.proofNonce,
            da_oracle::DataRootTuple {
                height: U256::from(pfb_height),
                dataRoot: celestia_block_data_hash,
            },
            da_oracle::BinaryMerkleProof {
                key: U256::from(inclusion_proof.index),
                numLeaves: U256::from(inclusion_proof.total),
                sideNodes: inclusion_proof
                    .aunts
                    .iter()
                    .map(|node| B256::from_slice(node.as_ref()))
                    .collect(),
            },
        )
        .call()
        .await?;
    assert!(result, "on-chain attestation proof verification failed");
    info!("✅ on-chain attestation proof verified");

    let commit_tx = ethereum_client
        .get_transaction_by_hash(commit_tx_hash)
        .await?
        .unwrap();

    let shares_proof = SharesProof {
        data: tx
            .proof
            .data
            .iter()
            .map(|data| Bytes::copy_from_slice(data))
            .collect(),

        shareProofs: tx
            .proof
            .share_proofs
            .iter()
            .map(|proof| NamespaceMerkleMultiproof {
                beginKey: U256::from(proof.start),
                endKey: U256::from(proof.end),
                sideNodes: proof
                    .nodes
                    .iter()
                    .map(|node| NamespaceNode::from(node.as_ref()))
                    .collect(),
            })
            .collect(),

        namespace: Namespace::new(tx.proof.namespace_version, tx.proof.namespace_id.as_ref()),

        rowRoots: tx
            .proof
            .row_proof
            .row_roots
            .iter()
            .map(|root| NamespaceNode::from(root.as_ref()))
            .collect(),

        rowProofs: tx
            .proof
            .row_proof
            .proofs
            .iter()
            .map(|proof| BinaryMerkleProof {
                sideNodes: proof
                    .aunts
                    .iter()
                    .map(|node| B256::from_slice(node.as_ref()))
                    .collect(),
                key: U256::from(proof.index),
                numLeaves: U256::from(proof.total),
            })
            .collect(),

        attestationProof: AttestationProof {
            tupleRootNonce: event.proofNonce,

            tuple: DataRootTuple {
                height: U256::from(pfb_height),
                dataRoot: celestia_block_data_hash,
            },

            proof: BinaryMerkleProof {
                sideNodes: inclusion_proof
                    .aunts
                    .iter()
                    .map(|node| B256::from_slice(node.as_ref()))
                    .collect(),
                key: U256::from(inclusion_proof.index),
                numLeaves: U256::from(inclusion_proof.total),
            },
        },
    };

    // below is the main verification logic, will be put into guest.
    verify_celestia_block_included(
        pfb_height,
        event.dataCommitment, // this is the root
        commit_tx.input().as_ref(),
        &blobstream_vk,
    )?;
    verify_shares_to_data_root_tuple_root(event.dataCommitment, shares_proof)?;

    Ok(())
}
