use crate::da_oracle::{
    BLOBSTREAM_PROGRAM_VKEY, DATA_COMMITMENT_MAX, ProofOutputs, SP1BlobstreamCalls,
    commitHeaderRangeCall,
};
use crate::verifier;
use crate::verifier::{SharesProof, verifyCall};
use alloy_primitives::{Address, B256, Bytes, U256};
use alloy_sol_types::{SolCall, SolInterface, SolType};
use revm::context::TxEnv;
use revm::database::InMemoryDB;
use revm::state::{AccountInfo, Bytecode};
use revm::{Context, ExecuteCommitEvm, MainBuilder, MainContext};
use serde::{Deserialize, Serialize};
use sp1_verifier::{PLONK_VK_BYTES, PlonkVerifier};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestInput {
    pub celestia_block_data_commitment: B256,
    pub commit_tx_input: Bytes,
    pub share_proof: SharesProof,
}

pub fn validate(input: GuestInput) -> eyre::Result<()> {
    verify_celestia_block_included(
        input.share_proof.attestationProof.tuple.height.to(),
        input.celestia_block_data_commitment,
        &input.commit_tx_input,
    )?;
    verify_shares_to_data_root_tuple_root(input.celestia_block_data_commitment, input.share_proof)?;
    Ok(())
}

pub fn verify_celestia_block_included(
    celestia_block_height: u64,
    celestia_block_data_commitment: B256,
    commit_tx_input: &[u8],
) -> eyre::Result<()> {
    let SP1BlobstreamCalls::commitHeaderRange(commitHeaderRangeCall {
        proof: commit_header_range_proof,
        publicValues: public_values,
    }) = SP1BlobstreamCalls::abi_decode(commit_tx_input)?
    else {
        eyre::bail!("unexpected commit tx call data");
    };

    let proof_outputs = ProofOutputs::abi_decode(public_values.as_ref())?;
    assert!(proof_outputs.targetBlock - proof_outputs.trustedBlock <= DATA_COMMITMENT_MAX);
    assert!(proof_outputs.trustedBlock <= celestia_block_height);
    assert!(proof_outputs.targetBlock >= celestia_block_height);
    assert_eq!(proof_outputs.dataCommitment, celestia_block_data_commitment);

    // verify commitHeaderRange proof
    PlonkVerifier::verify(
        commit_header_range_proof.as_ref(),
        public_values.as_ref(),
        BLOBSTREAM_PROGRAM_VKEY,
        PLONK_VK_BYTES.as_ref(),
    )?;

    Ok(())
}

pub fn verify_shares_to_data_root_tuple_root(
    root: B256,
    share_proof: SharesProof,
) -> eyre::Result<()> {
    let mut db = InMemoryDB::default();
    const VERIFIER_ADDRESS: Address = Address::repeat_byte(0x42);
    const CALLER_ADDRESS: Address = Address::repeat_byte(0xcc);
    db.insert_account_info(
        VERIFIER_ADDRESS,
        AccountInfo::from_bytecode(Bytecode::new_raw(verifier::DEPLOYED_BYTECODE.clone())),
    );
    db.insert_account_info(CALLER_ADDRESS, AccountInfo::from_balance(U256::MAX));

    let mut evm = Context::mainnet().with_db(db).build_mainnet();

    let input = verifyCall::new((root, share_proof)).abi_encode();

    let tx = TxEnv::builder()
        .caller(CALLER_ADDRESS)
        .to(VERIFIER_ADDRESS)
        .data(Bytes::from(input))
        .build()
        .unwrap();
    let result = evm.transact_commit(tx)?;
    assert!(result.is_success());

    Ok(())
}
