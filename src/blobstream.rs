use alloy::primitives::B256;
use alloy::providers::Provider;

alloy::sol! {
     #[sol(rpc)]
     #[derive(Debug)]
     contract SP1Blobstream {
        function commitHeaderRange(bytes calldata proof, bytes calldata publicValues) external {}

        uint64 public constant DATA_COMMITMENT_MAX;
        bytes32 public blobstreamProgramVkey;

        struct ProofOutputs {
            bytes32 trustedHeaderHash;
            bytes32 targetHeaderHash;
            bytes32 dataCommitment;
            uint64 trustedBlock;
            uint64 targetBlock;
            uint256 validatorBitmap;
        }

        event DataCommitmentStored(
            uint256 proofNonce,
            uint64 indexed startBlock,
            uint64 indexed endBlock,
            bytes32 indexed dataCommitment
        );
    }

    #[derive(Debug)]
    struct DataRootTuple {
        // Celestia block height the data root was included in.
        // Genesis block is height = 0.
        // First queryable block is height = 1.
        uint256 height;
        // Data root.
        bytes32 dataRoot;
    }
}

pub use SP1Blobstream::*;

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
