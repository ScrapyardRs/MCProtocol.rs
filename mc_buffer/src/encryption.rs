use std::io::{Cursor, Read};

use aes::Aes128;

use cfb8::cipher::{AsyncStreamCipher, NewCipher};
use cfb8::Cfb8;
use flate2::bufread::{ZlibDecoder, ZlibEncoder};
use flate2::Compression;

use mc_serializer::primitive::VarInt;
use mc_serializer::serde::{ProtocolVersion, Serialize};

pub type EncryptionStream = Cfb8<Aes128>;

pub struct Codec {
    encryption_stream: EncryptionStream,
}

impl Codec {
    pub fn new(shared_secret_bytes: &[u8]) -> anyhow::Result<(Self, Self)> {
        let (stream_read, stream_write) = (
            EncryptionStream::new_from_slices(shared_secret_bytes, shared_secret_bytes),
            EncryptionStream::new_from_slices(shared_secret_bytes, shared_secret_bytes),
        );
        match (stream_read, stream_write) {
            (Ok(stream_read), Ok(stream_write)) => Ok((
                Codec {
                    encryption_stream: stream_read,
                },
                Codec {
                    encryption_stream: stream_write,
                },
            )),
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

    pub fn encrypt(&mut self, bytes: &mut [u8]) {
        self.encryption_stream.encrypt(bytes)
    }

    pub fn decrypt(&mut self, bytes: &mut [u8]) {
        self.encryption_stream.decrypt(bytes)
    }
}

pub struct Compressor {
    threshold: VarInt,
}

impl Compressor {
    pub fn new(threshold: VarInt) -> Self {
        Self { threshold }
    }

    pub fn compress(&self, mut packet: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let initial_size = VarInt::try_from(packet.len())?;
        if initial_size >= self.threshold {
            let mut encoder = ZlibEncoder::new(packet.as_slice(), Compression::default());
            let mut compressed = Vec::new();
            encoder.read_to_end(&mut compressed)?;

            let compressed_length = VarInt::try_from(compressed.len())?;
            let uncompressed_length_length = initial_size.size(ProtocolVersion::Unknown)?;
            let total_length_data = compressed_length + VarInt::from(uncompressed_length_length);

            let mut result = Cursor::new(Vec::with_capacity(
                (total_length_data.size(ProtocolVersion::Unknown)? + uncompressed_length_length).try_into()?,
            ));
            total_length_data.serialize(&mut result, ProtocolVersion::Unknown)?;
            initial_size.serialize(&mut result, ProtocolVersion::Unknown)?;

            let mut inner = result.into_inner();
            inner.append(&mut compressed);
            Ok(inner)
        } else {
            let total_length_data = VarInt::from(1i32 /* 0 = 1 byte */) + initial_size;

            let mut result = Vec::with_capacity((total_length_data.size(ProtocolVersion::Unknown)?).try_into()?);
            total_length_data.serialize(&mut result, ProtocolVersion::Unknown)?;
            VarInt::from(0).serialize(&mut result, ProtocolVersion::Unknown)?;
            result.append(&mut packet);
            Ok(result)
        }
    }

    pub fn decompress(&self, packet: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let mut cursor = Cursor::new(packet);
        let (decompressed_length_size, decompressed_length) = VarInt::decode_and_size(&mut cursor)?;
        let remaining_bytes = &cursor.into_inner()[decompressed_length_size.try_into()?..];

        Ok(if decompressed_length == 0 {
            Vec::from(remaining_bytes)
        } else {
            let mut target = Vec::with_capacity(decompressed_length.try_into()?);
            ZlibDecoder::new(remaining_bytes).read_to_end(&mut target)?;
            target
        })
    }

    pub fn uncompressed(mut packet: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let length = VarInt::try_from(packet.len())?;
        let length_size = length.size(ProtocolVersion::Unknown)?;
        let mut writer = Vec::with_capacity((length + length_size).try_into()?);
        length.serialize(&mut writer, ProtocolVersion::Unknown)?;
        writer.append(&mut packet);
        Ok(writer)
    }
}

pub struct Encrypt {
    encryption: Codec,
}

impl Encrypt {
    pub fn new(encryption: Codec) -> Self {
        Self { encryption }
    }

    pub fn encrypt(&mut self, slice: &mut [u8]) {
        self.encryption.encrypt(slice);
    }
}

pub struct Decrypt {
    decryption: Codec,
}

impl Decrypt {
    pub fn new(decryption: Codec) -> Self {
        Self { decryption }
    }

    pub fn decrypt(&mut self, slice: &mut [u8]) {
        self.decryption.decrypt(slice);
    }
}
