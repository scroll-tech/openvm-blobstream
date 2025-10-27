use crate::{verifier, verifier::verifyCall};
use alloy_primitives::{Address, Bytes, U256, hex, keccak256};
use alloy_sol_types::{SolCall, SolValue};
use eyre::eyre;
use revm::{
    Context, ExecuteCommitEvm, MainBuilder, MainContext,
    context::{
        TxEnv,
        result::{ExecutionResult, Output},
    },
    database::InMemoryDB,
    state::{AccountInfo, Bytecode},
};

pub fn validate(input: verifyCall) -> eyre::Result<()> {
    const VERIFIER_ADDRESS: Address = Address::repeat_byte(0x42);
    const CALLER_ADDRESS: Address = Address::repeat_byte(0xcc);

    let mut db = InMemoryDB::default();

    db.insert_account_info(
        VERIFIER_ADDRESS,
        AccountInfo::from_bytecode(Bytecode::new_raw(verifier::DEPLOYED_BYTECODE.clone())),
    );
    db.insert_account_info(CALLER_ADDRESS, AccountInfo::from_balance(U256::MAX));

    let mut evm = Context::mainnet().with_db(db).build_mainnet();

    let input = input.abi_encode();
    let expected_commit = keccak256(&input[4..]);
    let tx = TxEnv::builder()
        .caller(CALLER_ADDRESS)
        .to(VERIFIER_ADDRESS)
        .data(Bytes::from(input))
        .build()
        .unwrap();
    let result = evm.transact_commit(tx)?;

    match result {
        ExecutionResult::Success {
            output: Output::Call(output),
            ..
        } if expected_commit.as_slice() == output.as_ref() => Ok(()),
        ExecutionResult::Revert { output, .. } if output.starts_with(&hex!("08c379a0")) => {
            let reason = String::abi_decode(&output[4..])?;
            Err(eyre!("EVM reverted: {reason}"))
        }
        other => Err(eyre!("execution failed: {other:?}")),
    }
}
