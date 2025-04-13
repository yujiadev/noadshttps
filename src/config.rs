use std::fs;
use std::net::SocketAddr;
use serde::Deserialize;
use toml::Value;

#[derive(Debug, Clone)]
pub struct ProxyConfigs {
	pub addr: SocketAddr,
	pub x_fwd_addr: SocketAddr,
	pub db_uri: String,
	pub blkls: String,
}

pub fn parse_configs(path: &str) -> ProxyConfigs {
	let raw = match std::fs::read_to_string(path) {
		Ok(c_str) => c_str,
		Err(e) => panic!("parse_configs(), {:?}", e),
	};

	let toml: Value = match toml::from_str(&raw) {
		Ok(values) => values,
		Err(e) => panic!("parse_configs(), {:?}", e),
	};

	let address: SocketAddr = toml["address"]
		.as_str()
		.expect("Invalid HTTPS proxy addr, needs to be String")
		.parse()
		.expect("Invalid HTTPS proxy addr, needs to an valid IP:POST addr");

	let x_forward_address: SocketAddr = toml["x_forward_address"]
		.as_str()
		.expect("Invalid X-Forward-Address, needs to be String")
		.parse()
		.expect("Invalid X-Forward-Address, needs to an valid IP:POST addr");

	let database_uri: String = toml["database_uri"]
		.as_str()
		.expect("Invalid SQLite databased URI")
		.to_string();

	let blocklist: String = toml["blocklist"]
		.as_str()
		.expect("Invalid path")
		.to_string();

	ProxyConfigs {
		addr: address,
		x_fwd_addr: x_forward_address,
		db_uri: database_uri,
		blkls: blocklist,
	}	
}