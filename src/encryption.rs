use aes::cipher::KeyIvInit;

pub type Aes128Cfb8Enc = cfb8::Encryptor<aes::Aes128>;
pub type Aes128Cfb8Dec = cfb8::Decryptor<aes::Aes128>;

pub fn encryption_from_secret(
    shared_secret_bytes: &[u8],
) -> anyhow::Result<(Aes128Cfb8Enc, Aes128Cfb8Dec)> {
    let (stream_read, stream_write) = (
        Aes128Cfb8Enc::new_from_slices(shared_secret_bytes, shared_secret_bytes),
        Aes128Cfb8Dec::new_from_slices(shared_secret_bytes, shared_secret_bytes),
    );
    match (stream_read, stream_write) {
        (Ok(stream_read), Ok(stream_write)) => Ok((stream_read, stream_write)),
        (Err(error), Ok(_)) => {
            anyhow::bail!("Failed to create read stream {}.", error);
        }
        (Ok(_), Err(error)) => {
            anyhow::bail!("Failed to create write stream {}.", error);
        }
        (Err(error), Err(error2)) => {
            anyhow::bail!("Failed to create both streams {}, {}.", error, error2);
        }
    }
}

pub fn encryption_from_response(
    response_verify: &[u8],
    shared_secret: &[u8],
    verify: &[u8],
) -> anyhow::Result<(Aes128Cfb8Enc, Aes128Cfb8Dec)> {
    if verify.ne(response_verify) {
        anyhow::bail!("Failed to assert verify token match.");
    }
    encryption_from_secret(shared_secret)
}
