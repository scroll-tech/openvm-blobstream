use alloy::consensus::Transaction;
use alloy::hex;
use alloy::primitives::{address, Address, B256};
use alloy::providers::Provider;
use clap::Parser;
use openvm_benchmarks_prove::util::BenchmarkCli;
use openvm_blobstream::da_oracle::{SP1Blobstream, BLOBSTREAM_PROGRAM_VKEY, DATA_COMMITMENT_MAX};
use openvm_blobstream::guest::GuestInput;
use openvm_blobstream::host::{
    find_commit_tx, get_celestia_data_root_inclusion_proof, get_celestia_tx_by_hash,
};
use openvm_blobstream::verifier::SharesProof;
use openvm_circuit::arch::instructions::exe::VmExe;
use openvm_sdk::config::{AppConfig, SdkVmConfig};
use openvm_sdk::{StdIn, F};
use openvm_stark_sdk::bench::run_with_metric_collection;
use tendermint_rpc::Client;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

pub const ELF: &[u8] = include_bytes!("../../target/openvm/release/blobstream-program.vmexe");
pub const CELESTIA_RPC_URL: &str = "https://celestia-rpc.publicnode.com:443";
pub const ETHEREUM_RPC_URL: &str = "https://ethereum-rpc.publicnode.com";
pub const BLOBSTREAM_CONTRACT_ADDRESS: Address =
    address!("0x7Cf3876F681Dbb6EdA8f6FfC45D66B996Df08fAe");
const TX_HASH: &[u8] = &hex!("38D01D9A80A1FB6D6550E0B8C6487AF229A0F6741B6AD84E7B2208819E80214C");

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let args: BenchmarkCli = BenchmarkCli::parse();

    let ethereum_client =
        alloy::providers::ProviderBuilder::new().connect_http(ETHEREUM_RPC_URL.parse()?);
    let celestia_client = tendermint_rpc::HttpClient::new(CELESTIA_RPC_URL)?;
    let blobstream_contract =
        SP1Blobstream::new(BLOBSTREAM_CONTRACT_ADDRESS, ethereum_client.clone());

    let blobstream_vk = blobstream_contract.blobstreamProgramVkey().call().await?;
    let blobstream_vk = hex::encode_prefixed(blobstream_vk);
    assert_eq!(blobstream_vk, BLOBSTREAM_PROGRAM_VKEY);
    info!("Blobstream VK hash: {blobstream_vk}");

    let data_commitment_max = blobstream_contract.DATA_COMMITMENT_MAX().call().await?;
    assert_eq!(data_commitment_max, DATA_COMMITMENT_MAX);

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

    let guest_inputs = GuestInput {
        celestia_block_data_commitment: event.dataCommitment,
        commit_tx_input: commit_tx.input().clone(),
        share_proof: SharesProof::new(
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

    let app_exe: VmExe<F> = bitcode::deserialize(ELF)?;

    run_with_metric_collection("OUT_PATH", || {
        let mut stdin = StdIn::default();
        stdin.write(&guest_inputs);

        args.bench_from_exe(
            "blobstream",
            app_config.app_vm_config.clone(),
            app_exe.clone(),
            stdin,
        )
    })?;

    Ok(())
}
