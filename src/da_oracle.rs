use alloy_sol_types::sol;

#[cfg(feature = "host")]
sol! {
    #[sol(rpc)]
    #[derive(Debug)]
    contract SP1Blobstream {
        function commitHeaderRange(bytes calldata proof, bytes calldata publicValues) external {}

        event DataCommitmentStored(
            uint256 proofNonce,
            uint64 indexed startBlock,
            uint64 indexed endBlock,
            bytes32 indexed dataCommitment
        );
    }
}

#[cfg(not(feature = "host"))]
sol! {
    #[derive(Debug)]
    contract SP1Blobstream {
        function commitHeaderRange(bytes calldata proof, bytes calldata publicValues) external {}
    }
}

pub use SP1Blobstream::*;
