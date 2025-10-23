use alloy_sol_types::sol;

pub const DATA_COMMITMENT_MAX: u64 = 10000;
pub const BLOBSTREAM_PROGRAM_VKEY: &str =
    "0x00de39c136b88dfeacb832629e21a9667935bc0e74aaa21292e4f237d79d0bef";

#[cfg(feature = "host")]
sol! {
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
}

#[cfg(not(feature = "host"))]
sol! {
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
}

pub use SP1Blobstream::*;
