use openvm_blobstream::guest::GuestInput;

openvm::entry!(main);
openvm::init!();

fn main() {
    openvm_blobstream::guest::validate(openvm::io::read::<GuestInput>()).unwrap();
}
