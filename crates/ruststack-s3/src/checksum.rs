use base64::Engine;
use ruststack_core::RustStackError;
use serde::{Deserialize, Serialize};
use sha2::Digest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChecksumAlgorithm {
    Crc32,
    Crc32c,
    Sha1,
    Sha256,
}

impl ChecksumAlgorithm {
    pub fn from_str_opt(s: &str) -> Option<Self> {
        if s.eq_ignore_ascii_case("CRC32") {
            Some(Self::Crc32)
        } else if s.eq_ignore_ascii_case("CRC32C") {
            Some(Self::Crc32c)
        } else if s.eq_ignore_ascii_case("SHA1") || s.eq_ignore_ascii_case("SHA-1") {
            Some(Self::Sha1)
        } else if s.eq_ignore_ascii_case("SHA256") || s.eq_ignore_ascii_case("SHA-256") {
            Some(Self::Sha256)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Crc32 => "CRC32",
            Self::Crc32c => "CRC32C",
            Self::Sha1 => "SHA1",
            Self::Sha256 => "SHA256",
        }
    }

    pub fn header_name(&self) -> &'static str {
        match self {
            Self::Crc32 => "x-amz-checksum-crc32",
            Self::Crc32c => "x-amz-checksum-crc32c",
            Self::Sha1 => "x-amz-checksum-sha1",
            Self::Sha256 => "x-amz-checksum-sha256",
        }
    }
}

// -----------------------------------------------------------------------------
// CRC32 (IEEE 802.3)
// -----------------------------------------------------------------------------
const fn make_crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

const CRC32_TABLE: [u32; 256] = make_crc32_table();

pub fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        let idx = ((crc ^ (b as u32)) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32_TABLE[idx];
    }
    crc ^ 0xFFFF_FFFFu32
}

// -----------------------------------------------------------------------------
// CRC32C (Castagnoli)
// -----------------------------------------------------------------------------
const fn make_crc32c_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0x82F6_3B78;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

const CRC32C_TABLE: [u32; 256] = make_crc32c_table();

pub fn compute_crc32c(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        let idx = ((crc ^ (b as u32)) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32C_TABLE[idx];
    }
    crc ^ 0xFFFF_FFFFu32
}

// -----------------------------------------------------------------------------
// SHA-1 (RFC 3174)
// -----------------------------------------------------------------------------
#[allow(unknown_lints)]
#[allow(clippy::chunks_exact_to_as_chunks)]
pub fn compute_sha1(data: &[u8]) -> [u8; 20] {
    let mut h0 = 0x6745_2301u32;
    let mut h1 = 0xEFCD_AB89u32;
    let mut h2 = 0x98BA_DCFEu32;
    let mut h3 = 0x1032_5476u32;
    let mut h4 = 0xC3D2_E1F0u32;

    let bit_len = (data.len() as u64).wrapping_mul(8);

    // Buffer with padding
    let mut msg = Vec::with_capacity(data.len() + 64);
    msg.extend_from_slice(data);
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    let chunk_size = 64;
    for chunk in msg.chunks_exact(chunk_size) {
        let mut w = [0u32; 80];
        let word_size = 4;
        for (slot, block) in w[..16].iter_mut().zip(chunk.chunks_exact(word_size)) {
            *slot = u32::from_be_bytes([block[0], block[1], block[2], block[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for (i, &item) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDCu32),
                _ => (b ^ c ^ d, 0xCA62_C1D6u32),
            };

            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(item);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

// -----------------------------------------------------------------------------
// SHA-256
// -----------------------------------------------------------------------------
pub fn compute_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

// -----------------------------------------------------------------------------
// Checksum Calculation and Validation
// -----------------------------------------------------------------------------
pub fn calculate_checksum(algorithm: ChecksumAlgorithm, data: &[u8]) -> String {
    let engine = base64::engine::general_purpose::STANDARD;
    match algorithm {
        ChecksumAlgorithm::Crc32 => engine.encode(compute_crc32(data).to_be_bytes()),
        ChecksumAlgorithm::Crc32c => engine.encode(compute_crc32c(data).to_be_bytes()),
        ChecksumAlgorithm::Sha1 => engine.encode(compute_sha1(data)),
        ChecksumAlgorithm::Sha256 => engine.encode(compute_sha256(data)),
    }
}

pub fn verify_checksum(
    algorithm: ChecksumAlgorithm,
    data: &[u8],
    expected_b64: &str,
) -> Result<(), RustStackError> {
    let computed = calculate_checksum(algorithm, data);
    if computed.trim() == expected_b64.trim() {
        Ok(())
    } else {
        Err(RustStackError::S3 {
            code: "BadDigest".to_string(),
            message: "The Content-MD5 or checksum value that you specified did not match what we received.".to_string(),
            status: http::StatusCode::BAD_REQUEST,
            resource: None,
        })
    }
}

/// Computes the AWS S3 composite multipart checksum.
///
/// Each part checksum is decoded into raw bytes and concatenated in ascending part order.
/// The algorithm computes the checksum over the concatenated binary checksums, base64-encodes it,
/// and appends `-[part_count]`.
pub fn compute_composite_checksum(
    algorithm: ChecksumAlgorithm,
    part_checksums: &[String],
) -> Result<String, RustStackError> {
    if part_checksums.is_empty() {
        return Ok(String::new());
    }

    let engine = base64::engine::general_purpose::STANDARD;
    let mut concatenated = Vec::new();

    for cs in part_checksums {
        // Strip any existing "-N" suffix if nested
        let clean = cs.split('-').next().unwrap_or(cs).trim();
        let bytes = engine.decode(clean).map_err(|e| {
            RustStackError::BadRequest(format!("Invalid base64 checksum '{}': {}", cs, e))
        })?;
        concatenated.extend_from_slice(&bytes);
    }

    let raw_checksum = calculate_checksum(algorithm, &concatenated);
    Ok(format!("{}-{}", raw_checksum, part_checksums.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32() {
        assert_eq!(compute_crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(
            calculate_checksum(ChecksumAlgorithm::Crc32, b"Hello world"),
            "i9aeUg=="
        );
    }

    #[test]
    fn test_crc32c() {
        assert_eq!(compute_crc32c(b"123456789"), 0xE306_9283);
        assert_eq!(
            calculate_checksum(ChecksumAlgorithm::Crc32c, b"Hello world"),
            "crUfeA=="
        );
    }

    #[test]
    fn test_sha1() {
        let digest = compute_sha1(b"abc");
        assert_eq!(
            hex::encode(digest),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }

    #[test]
    fn test_sha256() {
        let digest = compute_sha256(b"abc");
        assert_eq!(
            hex::encode(digest),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_composite_checksum() {
        let part1 = calculate_checksum(ChecksumAlgorithm::Crc32, b"Part 1 data");
        let part2 = calculate_checksum(ChecksumAlgorithm::Crc32, b"Part 2 data");

        let composite =
            compute_composite_checksum(ChecksumAlgorithm::Crc32, &[part1, part2]).unwrap();
        assert!(composite.ends_with("-2"));
        assert!(!composite.is_empty());
    }
}
