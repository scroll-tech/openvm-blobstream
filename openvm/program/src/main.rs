extern crate openvm_keccak256_guest;

use openvm_blobstream::GuestInput;

openvm::entry!(main);
openvm::init!();

fn main() {
    openvm_blobstream::install_revm_crypto(openvm_blobstream::openvm::Crypto);
    openvm_blobstream::guest::validate(openvm::io::read::<GuestInput>()).unwrap();
}
