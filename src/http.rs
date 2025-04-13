use std::io::{ Result, ErrorKind, Error };
use std::time::Duration;
use regex::Regex;
use tokio::time::timeout;
use tokio::io::{ AsyncReadExt, AsyncWriteExt, copy_bidirectional };
use tokio::net::{ TcpListener, TcpStream };
use tracing::{ debug, error, info, span, warn, Level };

const OVERFLOW_LIMIT: usize = 4096;
const UNDERFLOW_LIMIT: usize = 32;
const TWO_SECS: Duration = Duration::from_secs(2);

pub const CRLF_BYTES: &[u8; 2] = b"\r\n";
pub const CRLF2_BYTES: &[u8; 4] = b"\r\n\r\n";
pub const CONNECT_RESPONSE: &[u8; 39] = b"HTTP/1.1 200 Connection Established\r\n\r\n";
pub const BAD_REQUEST: &[u8; 28] = b"HTTP/1.1 400 Bad Request\r\n\r\n";

pub fn parse_connect_request_host(buffer: &[u8]) -> Result<Option<String>> {
	// Insufficient bytes to parse, return
	if buffer.len() <= UNDERFLOW_LIMIT {
		return Ok(None);
	};

	// Return if the HTTP verb is not "CONNECT" 
	if &buffer[0..7] != b"CONNECT" {
		return Err(Error::new(ErrorKind::InvalidData, "Not HTTP CONNECT verb"));
	}

	let (mut start, mut end, mut index) = (0, 0, 0);
	let mut host: Option<String> = None;

	// Prase the request line of the HTTP connect request.
	loop {
		if index > OVERFLOW_LIMIT {
			return Err(Error::new(
				ErrorKind::InvalidData, 
				"HTTP connect request exceeds maximum request size",
			));
		}

		// Run out of bytes to find, return 
		if buffer[index..].len() < CRLF_BYTES.len() {
			return Ok(None)
		}

		// Check if the pattern is matched
		start = index;
		end = index + CRLF_BYTES.len();
		if &buffer[start..end] != &CRLF_BYTES[..] {
			index += 1;
			continue;
		}

		// Update the index for the second loop, the second loop will use the 
		// index to continue parse rest of bytes.
		index += 1;
		break;
	}

	// Prase the host address.
	host = match String::from_utf8(buffer[0..end].to_vec()) {
		Ok(line) => {
			let segments: Vec<_> = line.split(' ').collect();

			// The structure should be "CONNECT domain:port HTTP/1.1"
			if segments.len() != 3 {
				return Err(Error::new(
					ErrorKind::InvalidData, 
					"HTTP connect request line is malformat",
				));
			}

			// Check if the host format is correct.
    		let host_format_pattern = Regex::new(
    			r"^([a-zA-Z0-9.-]+|\d{1,3}(\.\d{1,3}){3}):\d+$"
    		).unwrap();

    		if !host_format_pattern.is_match(&segments[1]) {
				return Err(Error::new(
					ErrorKind::InvalidData, 
					"HTTP connect request line is malformat",
				));
    		}

			Some(segments[1].to_string())
		},
		Err(_) => {
			return Err(Error::new(
				ErrorKind::InvalidData, 
				"HTTP connect request line can't be parsed from utf-8",
			));
		},
	};

	// Check the rest of the lines of HTTP connect request.
	loop {
		if index > OVERFLOW_LIMIT {
			return Err(Error::new(
				ErrorKind::InvalidData, 
				"HTTP connect request exceeds maximum request size",
			));
		}

		// Run out of bytes to find, return 
		if buffer[index..].len() < CRLF2_BYTES.len() {
			return Ok(None)
		}

		// Check if the pattern is matched
		start = index;
		end = index + CRLF2_BYTES.len();
		if &buffer[start..end] != &CRLF2_BYTES[..] {
			index += 1;
			continue;
		}

		break;
	}

	return Ok(host);
}

pub async fn parse_connect_request_from_stream(
	stream: &mut TcpStream,
) -> Result<String> 
{
	let mut buf = [0u8; OVERFLOW_LIMIT];
	let mut nbytes = 0;
	let mut host = String::new();

	loop {
		let _ = match stream.read(&mut buf).await {
			Ok(0) => return Err(Error::new(
				ErrorKind::UnexpectedEof,
				"parse_connect_request_from_stream encounter unexpected EOF",
			)),
			Ok(n) => nbytes += n,
			Err(e) => return Err(e),
		};

		let _ = match parse_connect_request_host(&buf) {
			Ok(None) => continue,
			Ok(Some(addr)) => host = addr,
			Err(e) => return Err(e),
		};

		break;
	}

	Ok(host)
}

pub async fn read_connect_request(stream: &mut TcpStream) -> Result<Vec<u8>> {
	let mut buf = [0u8; OVERFLOW_LIMIT];
	let mut nbytes = 0;
	let mut request = Vec::new();

	loop {
		let _ = match timeout(TWO_SECS, stream.read(&mut buf)).await {
			Ok(Ok(0)) => return Err(Error::new(
				ErrorKind::UnexpectedEof,
				"parse_connect_request_from_stream encounter unexpected EOF",
			)),
			Ok(Ok(n)) => nbytes += n,
			Ok(Err(e)) => return Err(e),
			Err(e) => return Err(Error::other("read_connect_request(), timeout")),
		};

		// Not enough bytes to read a complete ONNECT request, keep reading
		if nbytes < UNDERFLOW_LIMIT {
			continue;
		}

		// If the last 4 bytes ain't CRLF2, keep reading
		if &buf[nbytes-4..nbytes] != CRLF2_BYTES {
			continue;
		}

		request.extend_from_slice(&buf[..nbytes]);
		break;
	}

	return Ok(request);
}

pub fn get_domain_from_host(host: &str) -> Result<String> {
	let _ = match host.rfind(':') {
		Some(index) => {
			let (domain, _) = host.split_at(index);
			return Ok(domain.to_string());
		},
		None => {
			return Err(Error::new(
				ErrorKind::InvalidData, 
				"URL should be domain:port",
			));	
		},
	};	
}