// SPDX-License-Identifier: MIT
// solhint-disable gas-custom-errors
pragma solidity ^0.8.30;

import {DAVerifier, SharesProof} from "verifier/DAVerifier.sol";
import {IDAOracle} from "blobstream-contracts/IDAOracle.sol";
import {DataRootTuple} from "blobstream-contracts/DataRootTuple.sol";
import {BinaryMerkleProof} from "tree/binary/BinaryMerkleProof.sol";
import {BinaryMerkleTree} from "tree/binary/BinaryMerkleTree.sol";
import {SP1Verifier} from "sp1-contracts/src/v5.0.0/SP1VerifierPlonk.sol";

contract Verifier is IDAOracle, SP1Verifier {
    uint64 public constant DATA_COMMITMENT_MAX = 10000;
    bytes32 public constant SP1_BLOB_STREAM_PROGRAM_VK =
        0x00de39c136b88dfeacb832629e21a9667935bc0e74aaa21292e4f237d79d0bef;

    bytes32 internal root;
    uint256 internal tupleRootNonce;

    struct ProofOutputs {
        bytes32 trustedHeaderHash;
        bytes32 targetHeaderHash;
        bytes32 dataCommitment;
        uint64 trustedBlock;
        uint64 targetBlock;
        uint256 validatorBitmap;
    }

    function verify(
        bytes calldata commitHeaderRangeProof,
        bytes calldata commitHeaderRangePublicValues,
        SharesProof calldata sharesProof
    ) external returns (bytes32 commit) {
        ProofOutputs memory po = abi.decode(commitHeaderRangePublicValues, (ProofOutputs));

        uint64 blockHeight = uint64(sharesProof.attestationProof.tuple.height);
        uint64 startBlock = po.trustedBlock;
        uint64 endBlock = po.targetBlock;

        require(endBlock - startBlock <= DATA_COMMITMENT_MAX, "proof block range too large");
        require(blockHeight >= startBlock && blockHeight <= endBlock, "attested block not in range");

        root = po.dataCommitment;
        tupleRootNonce = sharesProof.attestationProof.tupleRootNonce;

        verifyBlobStreamProof(commitHeaderRangePublicValues, commitHeaderRangeProof);

        (bool committedTo,) = DAVerifier.verifySharesToDataRootTupleRoot(this, sharesProof);
        require(committedTo, "invalid sharesProof");

        // hash inputs
        assembly {
            let cds := calldatasize()
            let n := sub(cds, 4)
            calldatacopy(0x00, 4, n)
            commit := keccak256(0x00, n)
        }
    }

    function verifyAttestation(uint256 _tupleRootNonce, DataRootTuple memory tuple, BinaryMerkleProof memory proof)
        external
        view
        returns (bool)
    {
        require(_tupleRootNonce == tupleRootNonce, "invalid nonce");

        (bool isProofValid,) = BinaryMerkleTree.verify(root, proof, abi.encode(tuple));

        require(isProofValid, "invalid BinaryMerkleTree proof");
        return isProofValid;
    }

    function verifyBlobStreamProof(bytes calldata publicValues, bytes calldata proofBytes) internal view {
        bytes4 receivedSelector = bytes4(proofBytes[:4]);
        bytes4 expectedSelector = bytes4(VERIFIER_HASH());
        if (receivedSelector != expectedSelector) {
            revert WrongVerifierSelector(receivedSelector, expectedSelector);
        }

        bytes32 publicValuesDigest = hashPublicValues(publicValues);
        uint256[] memory inputs = new uint256[](2);
        inputs[0] = uint256(SP1_BLOB_STREAM_PROGRAM_VK);
        inputs[1] = uint256(publicValuesDigest);
        bool success = this.Verify(proofBytes[4:], inputs);
        require(success, "invalid commitHeaderRange proof");
    }
}
