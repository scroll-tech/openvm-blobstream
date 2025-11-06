use alloy::{
    consensus::Transaction,
    hex,
    primitives::{Address, B256, address},
    providers::Provider,
    sol_types::SolInterface,
};
use openvm_blobstream::{
    GuestInput,
    da_oracle::{SP1Blobstream, SP1BlobstreamCalls, commitHeaderRangeCall},
    host::{find_commit_tx, get_celestia_data_root_inclusion_proof, get_celestia_tx_by_hash},
    verifier::SharesProof,
};
use openvm_circuit::arch::instructions::exe::VmExe;
use openvm_sdk::{
    F, Sdk, StdIn,
    config::{AppConfig, SdkVmConfig},
    types::VersionedVmStarkProof,
};
use tendermint_rpc::Client;
use tracing::info;

pub const ELF: &[u8] = include_bytes!("../../target/openvm/release/blobstream-program.vmexe");
pub const CELESTIA_RPC_URL: &str = "https://celestia-rpc.publicnode.com:443";
pub const ETHEREUM_RPC_URL: &str = "https://ethereum-rpc.publicnode.com";
pub const BLOBSTREAM_CONTRACT_ADDRESS: Address =
    address!("0x7Cf3876F681Dbb6EdA8f6FfC45D66B996Df08fAe");
const TX_HASH: &[u8] = &hex!("526684971CD73022587E79D66B45049C7F824D08E8AE53FA9DB43CA45B55B446");

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let ethereum_client =
        alloy::providers::ProviderBuilder::new().connect_http(ETHEREUM_RPC_URL.parse()?);
    let celestia_client = tendermint_rpc::HttpClient::new(CELESTIA_RPC_URL)?;
    let blobstream_contract =
        SP1Blobstream::new(BLOBSTREAM_CONTRACT_ADDRESS, ethereum_client.clone());

    let tx = get_celestia_tx_by_hash(CELESTIA_RPC_URL, TX_HASH).await?;
    let pfb_height = tx.height;
    let celestia_block = celestia_client.block(pfb_height as u32).await?.block;
    let celestia_block_data_hash =
        B256::from_slice(celestia_block.header.data_hash.unwrap().as_ref());
    info!(
        "PayForBlobs tx at celestia height #{pfb_height} with data hash {celestia_block_data_hash}"
    );

    let (commit_tx_hash, event) =
        find_commit_tx(&blobstream_contract, &ethereum_client, pfb_height).await?;
    info!("found DataCommitmentStored event in ethereum tx {commit_tx_hash}: {event:?}");

    let inclusion_proof = get_celestia_data_root_inclusion_proof(
        CELESTIA_RPC_URL,
        pfb_height,
        event.startBlock,
        event.endBlock,
    )
    .await?;

    let commit_tx = ethereum_client
        .get_transaction_by_hash(commit_tx_hash)
        .await?
        .unwrap();

    let SP1BlobstreamCalls::commitHeaderRange(commitHeaderRangeCall {
        proof: commit_header_range_proof,
        publicValues: public_values,
    }) = SP1BlobstreamCalls::abi_decode(commit_tx.input().as_ref())?;

    let guest_inputs = GuestInput {
        commitHeaderRangeProof: commit_header_range_proof,
        commitHeaderRangePublicValues: public_values,
        sharesProof: SharesProof::new(
            pfb_height,
            celestia_block_data_hash,
            event.proofNonce,
            inclusion_proof,
            tx.proof,
        ),
    };

    openvm_blobstream::guest::validate(guest_inputs.clone())?; // run on host
    info!("verified successfully on host");

    let app_config: AppConfig<SdkVmConfig> =
        toml::from_str(include_str!("../../program/openvm.toml"))?;
    let sdk = Sdk::new(app_config.clone())?;

    let app_exe: VmExe<F> = bitcode::deserialize(ELF)?;

    let mut stdin = StdIn::default();
    stdin.write(&guest_inputs);

    let (_, (cost, instret)) = sdk.execute_metered_cost(app_exe.clone(), stdin.clone())?;
    info!("cells = {cost}, total_cycle = {instret}");

    let (proof, commit) = sdk.prove(app_exe.clone(), stdin.clone())?;
    let proof = VersionedVmStarkProof::new(proof)?;

    serde_json::to_writer_pretty(std::fs::File::create("app-commit.json")?, &commit)?;
    serde_json::to_writer_pretty(std::fs::File::create("blobstream.stark.proof")?, &proof)?;
    Ok(())
}
