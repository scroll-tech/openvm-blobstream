use alloy::{
    consensus::Transaction,
    hex,
    primitives::{Address, B256, address},
    providers::Provider,
    sol_types::SolInterface,
};
use clap::Parser;
use openvm_benchmarks_prove::util::BenchmarkCli;
use openvm_blobstream::{
    GuestInput,
    da_oracle::{SP1Blobstream, SP1BlobstreamCalls, commitHeaderRangeCall},
    host::{find_commit_tx, get_celestia_data_root_inclusion_proof, get_celestia_tx_by_hash},
    verifier::SharesProof,
};
use openvm_circuit::arch::instructions::exe::VmExe;
use openvm_sdk::{
    F, Sdk, StdIn,
    config::{AppConfig, SdkVmBuilder, SdkVmConfig},
};
use openvm_stark_sdk::bench::run_with_metric_collection;
use tendermint_rpc::Client;

pub const ELF: &[u8] = include_bytes!("../../target/openvm/release/blobstream-program.vmexe");
pub const CELESTIA_RPC_URL: &str = "https://celestia-rpc.publicnode.com:443";
pub const ETHEREUM_RPC_URL: &str = "https://ethereum-rpc.publicnode.com";
pub const BLOBSTREAM_CONTRACT_ADDRESS: Address =
    address!("0x7Cf3876F681Dbb6EdA8f6FfC45D66B996Df08fAe");
const TX_HASH: &[u8] = &hex!("38D01D9A80A1FB6D6550E0B8C6487AF229A0F6741B6AD84E7B2208819E80214C");

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args: BenchmarkCli = BenchmarkCli::parse();

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
    println!(
        "PayForBlobs tx at celestia height #{pfb_height} with data hash {celestia_block_data_hash}"
    );

    let (commit_tx_hash, event) =
        find_commit_tx(&blobstream_contract, &ethereum_client, pfb_height).await?;
    println!("found DataCommitmentStored event in ethereum tx {commit_tx_hash}: {event:?}");

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
    println!("verified successfully on host");

    let app_config: AppConfig<SdkVmConfig> =
        toml::from_str(include_str!("../../program/openvm.toml"))?;
    let sdk = Sdk::new(app_config.clone())?;

    let app_exe: VmExe<F> = bitcode::deserialize(ELF)?;

    let mut stdin = StdIn::default();
    stdin.write(&guest_inputs);

    let (_, (cost, instret)) = sdk.execute_metered_cost(app_exe.clone(), stdin.clone())?;
    println!("cells = {cost}, total_cycle = {instret}");

    run_with_metric_collection("OUT_PATH", || {
        args.bench_from_exe::<SdkVmBuilder, _>(
            "blobstream",
            app_config.app_vm_config.clone(),
            app_exe,
            stdin,
        )
    })?;

    Ok(())
}
