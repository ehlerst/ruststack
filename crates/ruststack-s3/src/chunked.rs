//! `aws-chunked` request-body decoding for S3 streaming uploads.
//!
//! AWS CLI/SDK uploads with default integrity protection (and all
//! `STREAMING-*-PAYLOAD*` signed-payload uploads) send the object data with
//! `Content-Encoding: aws-chunked` framing:
//!
//! ```text
//! <hex-size>[;chunk-signature=...]\r\n
//! <payload bytes>\r\n
//! ...
//! 0[;chunk-signature=...]\r\n
//! x-amz-checksum-<algo>:<base64>\r\n   <- optional trailer headers
//! \r\n
//! ```
//!
//! S3 stores only the decoded payload; the framing and the trailing checksum
//! must not leak into the stored object. Without this decoding, every
//! `aws s3 cp` upload (which defaults to `aws-chunked` + a CRC64NVME trailer
//! on CLI >= 2.x) would be stored corrupted.

use crate::checksum::{verify_checksum, ChecksumAlgorithm};
use ruststack_core::RustStackError;

/// Upper bound for a chunk-size line (`16^16` would need 17 chars; 32 leaves
/// ample room for the `;chunk-signature=` extension).
const MAX_SIZE_LINE_LEN: usize = 128;

/// Upper bound for a single trailer header line.
const MAX_TRAILER_LINE_LEN: usize = 1024;

fn bad_request(message: &str) -> RustStackError {
    RustStackError::s3_bad_request("InvalidRequest", message)
}

/// Reads one line (terminated by `\n`, optionally preceded by `\r`).
/// Returns the line contents without the terminator and the new position.
fn next_line(body: &[u8], pos: usize, max_len: usize) -> Result<(&[u8], usize), RustStackError> {
    if pos >= body.len() {
        return Err(bad_request("aws-chunked body ended unexpectedly"));
    }
    let end = body[pos..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|idx| pos + idx)
        .ok_or_else(|| bad_request("aws-chunked chunk line is not newline terminated"))?;
    let line_len = end - pos;
    if line_len > max_len {
        return Err(bad_request("aws-chunked line exceeds the maximum length"));
    }
    let mut line = &body[pos..end];
    if line.last() == Some(&b'\r') {
        line = &line[..line.len() - 1];
    }
    Ok((line, end + 1))
}

/// Parses a chunk-size line such as `b` (hex) or
/// `400;chunk-signature=abcdef...`.
fn parse_chunk_size(line: &[u8]) -> Result<u64, RustStackError> {
    let size_part = match line.iter().position(|&b| b == b';') {
        Some(idx) => &line[..idx],
        None => line,
    };
    if size_part.is_empty() || size_part.len() > 16 {
        return Err(bad_request("aws-chunked chunk size is invalid"));
    }
    let size_str = std::str::from_utf8(size_part)
        .map_err(|_| bad_request("aws-chunked chunk size is not valid hex"))?;
    u64::from_str_radix(size_str, 16)
        .map_err(|_| bad_request("aws-chunked chunk size is not valid hex"))
}

/// Decodes an `aws-chunked` body into the payload bytes and the trailer
/// headers (`name`, `value` pairs with names already lower-cased).
pub fn decode_aws_chunked(
    body: &[u8],
) -> Result<(bytes::Bytes, Vec<(String, String)>), RustStackError> {
    let mut decoded: Vec<u8> = Vec::with_capacity(body.len() / 2);
    let mut pos = 0usize;
    let mut trailers: Vec<(String, String)> = Vec::new();

    loop {
        let (line, next) = next_line(body, pos, MAX_SIZE_LINE_LEN)?;
        pos = next;
        let size = parse_chunk_size(line)?;

        if size == 0 {
            // Terminal chunk: remaining lines (if any) are trailer headers,
            // terminated by an empty line or the end of the body.
            while pos < body.len() {
                let (line, next) = next_line(body, pos, MAX_TRAILER_LINE_LEN)?;
                pos = next;
                if line.is_empty() {
                    break;
                }
                let header = std::str::from_utf8(line)
                    .map_err(|_| bad_request("aws-chunked trailer is not valid UTF-8"))?;
                if let Some((name, value)) = header.split_once(':') {
                    trailers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
                }
            }
            break;
        }

        let size = size as usize;
        let remaining = body.len().saturating_sub(pos);
        if size > remaining {
            return Err(bad_request(
                "aws-chunked chunk size exceeds the remaining body",
            ));
        }
        decoded.extend_from_slice(&body[pos..pos + size]);
        pos += size;
        // Every data chunk is followed by a CRLF (or bare LF).
        let (line, next) = next_line(body, pos, 1)?;
        pos = next;
        if !line.is_empty() {
            return Err(bad_request("aws-chunked chunk is not terminated by CRLF"));
        }
    }

    Ok((bytes::Bytes::from(decoded), trailers))
}

/// Verifies trailer checksum headers (`x-amz-checksum-<algo>`) against the
/// decoded payload. Non-checksum trailer headers are ignored.
pub fn verify_trailer_checksums(
    payload: &[u8],
    trailers: &[(String, String)],
) -> Result<(), RustStackError> {
    for (name, value) in trailers {
        let Some(algorithm) = ChecksumAlgorithm::from_header_name(name) else {
            continue;
        };
        verify_checksum(algorithm, payload, value)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_single_chunk_with_trailer() {
        let body = b"14\r\nmodel:\n  name: demo\n\r\n0\r\nx-amz-checksum-crc64nvme:AAAA\r\n\r\n";
        let (payload, trailers) = decode_aws_chunked(body).unwrap();
        assert_eq!(payload, b"model:\n  name: demo\n".as_slice());
        assert_eq!(trailers.len(), 1);
        assert_eq!(trailers[0].0, "x-amz-checksum-crc64nvme");
        assert_eq!(trailers[0].1, "AAAA");
    }

    #[test]
    fn test_decode_multiple_chunks_with_signature_extension() {
        let body = b"5;chunk-signature=deadbeefdeadbeef\r\nhello\r\n5;chunk-signature=deadbeefdeadbeef\r\nworld\r\n0\r\n";
        let (payload, trailers) = decode_aws_chunked(body).unwrap();
        assert_eq!(payload, b"helloworld".as_slice());
        assert!(trailers.is_empty());
    }

    #[test]
    fn test_decode_terminates_without_trailer() {
        let body = b"3\r\nabc\r\n0\r\n\r\n";
        let (payload, trailers) = decode_aws_chunked(body).unwrap();
        assert_eq!(payload, b"abc".as_slice());
        assert!(trailers.is_empty());
    }

    #[test]
    fn test_decode_rejects_invalid_chunk_size() {
        let body = b"xyz\r\nabc\r\n0\r\n";
        assert!(decode_aws_chunked(body).is_err());
    }

    #[test]
    fn test_decode_rejects_truncated_chunk() {
        let body = b"1000\r\nabc\r\n0\r\n";
        assert!(decode_aws_chunked(body).is_err());
    }

    #[test]
    fn test_decode_rejects_unterminated_body() {
        let body = b"3\r\nab";
        assert!(decode_aws_chunked(body).is_err());
    }

    #[test]
    fn test_verify_trailer_checksums_ok_and_mismatch() {
        let payload = b"Hello world";
        // CRC64NVME vector from the AWS CLI trailer (see checksum tests).
        let ok = vec![(
            "x-amz-checksum-crc64nvme".to_string(),
            "OOJZ0D8xKts=".to_string(),
        )];
        assert!(verify_trailer_checksums(payload, &ok).is_ok());

        let bad = vec![(
            "x-amz-checksum-crc64nvme".to_string(),
            "AAAAAAAAAAA=".to_string(),
        )];
        assert!(verify_trailer_checksums(payload, &bad).is_err());

        // Unknown/other trailer headers are ignored.
        let other = vec![("x-amz-trailer-status".to_string(), "x".to_string())];
        assert!(verify_trailer_checksums(payload, &other).is_ok());
    }
}
