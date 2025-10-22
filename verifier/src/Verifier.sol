// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {DAVerifier, SharesProof} from "verifier/DAVerifier.sol";
import {IDAOracle} from "blobstream-contracts/IDAOracle.sol";
import {DataRootTuple} from "blobstream-contracts/DataRootTuple.sol";
import {BinaryMerkleProof} from "tree/binary/BinaryMerkleProof.sol";
import {BinaryMerkleTree} from "tree/binary/BinaryMerkleTree.sol";

contract Verifier is IDAOracle {
    bytes32 root;
    uint256 tupleRootNonce;

    function verify(bytes32 _root, SharesProof calldata sharesProof) external {
        root = _root;
        tupleRootNonce = sharesProof.attestationProof.tupleRootNonce;
        (bool committedTo, ) = DAVerifier.verifySharesToDataRootTupleRoot(
            this,
            sharesProof
        );
        require(committedTo, "verifySharesToDataRootTupleRoot failed");
    }

    function verifyAttestation(
        uint256 _tupleRootNonce,
        DataRootTuple memory tuple,
        BinaryMerkleProof memory proof
    ) external view returns (bool) {
        require(_tupleRootNonce == tupleRootNonce, "Invalid nonce");

        (bool isProofValid, ) = BinaryMerkleTree.verify(
            root,
            proof,
            abi.encode(tuple)
        );

        require(isProofValid, "BinaryMerkleTree.verify failed");
        return isProofValid;
    }
}
