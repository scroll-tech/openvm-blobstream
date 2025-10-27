# blobstream verifier

this verifier verifies several things:

a. there was a range of blocks was witnessed by celetia validators
b. the block of tx contains the desired blob data in those blocks, aka. attestation proof
c. the blob data exists in the merkle tree of the block, aka. shares proof

The verifier is implemented in solidity and can be found in [`Verifier.sol`](./verifier/src/Verifier.sol).

## Guest Input

The `host` feature, provides utils to dump necessary data for constructing the guest input.

You can find the usage in [`openvm/script/src/main.rs`](./openvm/script/src/main.rs).

1. fetch the celestia tx which contains the blob data
2. fetch the block at `tx.height` to obtain the `data_hash`
3. find the `DataCommitmentStored` event contains this block, emit by Blobstream contract
4. get the transaction from ethereum that fires this event, obtain its call data, which contains the proof and public
   values.
   (this proves a.)
5. get the data root inclusion proof from celetia of the block at `tx.height` for the `data_hash`
   (this proves b.)
6. get the tx proof from celetia of this tx
   (this proves c.)
7. construct the guest input struct using all the above data.
