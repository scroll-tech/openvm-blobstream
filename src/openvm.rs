use revm::precompile::PrecompileError;

mod bn254;

#[derive(Debug, Copy, Clone, Default)]
pub struct Crypto;

impl crate::RevmCrypto for Crypto {
    #[inline]
    fn bn254_g1_add(&self, p1: &[u8], p2: &[u8]) -> Result<[u8; 64], PrecompileError> {
        let p1 = bn254::read_g1_point(p1)?;
        let p2 = bn254::read_g1_point(p2)?;
        let result = bn254::g1_point_add(p1, p2);
        Ok(bn254::encode_g1_point(result))
    }

    #[inline]
    fn bn254_g1_mul(&self, point: &[u8], scalar: &[u8]) -> Result<[u8; 64], PrecompileError> {
        let p = bn254::read_g1_point(point)?;
        let fr = bn254::read_scalar(scalar);
        let result = bn254::g1_point_mul(p, fr);
        Ok(bn254::encode_g1_point(result))
    }

    #[inline]
    fn bn254_pairing_check(&self, pairs: &[(&[u8], &[u8])]) -> Result<bool, PrecompileError> {
        bn254::pairing_check(pairs)
    }
}
