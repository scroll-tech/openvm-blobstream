use crate::DATA_COMMITMENT_MAX;
use crate::blobstream::{ProofOutputs, SP1BlobstreamCalls, commitHeaderRangeCall};
use crate::verifier::{SharesProof, verifyCall};
use alloy::primitives::{Address, B256, Bytes, U256};
use alloy::sol_types::{SolCall, SolInterface, SolType};
use revm::context::TxEnv;
use revm::database::InMemoryDB;
use revm::state::{AccountInfo, Bytecode};
use revm::{Context, ExecuteCommitEvm, MainBuilder, MainContext};
use sp1_verifier::{PLONK_VK_BYTES, PlonkVerifier};
use tracing::info;

pub fn verify_celestia_block_included(
    celestia_block_height: u64,
    celestia_block_data_commitment: B256,
    commit_tx_input: &[u8],
    blobstream_vk: &str,
) -> eyre::Result<()> {
    let SP1BlobstreamCalls::commitHeaderRange(commitHeaderRangeCall {
        proof: commit_header_range_proof,
        publicValues: public_values,
    }) = SP1BlobstreamCalls::abi_decode(commit_tx_input)?
    else {
        eyre::bail!("unexpected commit tx call data");
    };

    let proof_outputs = ProofOutputs::abi_decode(public_values.as_ref())?;
    info!("{proof_outputs:?}");
    assert!(proof_outputs.targetBlock - proof_outputs.trustedBlock <= DATA_COMMITMENT_MAX);
    info!("✅ targetBlock - trustedBlock <= DATA_COMMITMENT_MAX");
    assert!(proof_outputs.trustedBlock <= celestia_block_height);
    assert!(proof_outputs.targetBlock >= celestia_block_height);
    info!("✅ celestia_block_height in [trustedBlock, targetBlock]");
    assert_eq!(proof_outputs.dataCommitment, celestia_block_data_commitment);
    info!("✅ data_commitment matches event");

    // verify commitHeaderRange proof
    PlonkVerifier::verify(
        commit_header_range_proof.as_ref(),
        public_values.as_ref(),
        blobstream_vk,
        PLONK_VK_BYTES.as_ref(),
    )?;
    info!("✅ commitHeaderRange proof verified");

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
        AccountInfo::from_bytecode(Bytecode::new_raw(
            crate::verifier::DEPLOYED_BYTECODE.clone(),
        )),
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
    info!("EVM execution gas used: {:?}", result);
    assert!(result.is_success());
    info!("✅ shares to data root tuple root verified on EVM");

    Ok(())
}
