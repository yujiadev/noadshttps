use std::io::{ Result, Error, ErrorKind };
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::io::{ AsyncReadExt, AsyncWriteExt, copy_bidirectional };
use tokio::net::{ TcpListener, TcpStream };
use tokio::sync::Semaphore;
use crate::http::{ 
	parse_connect_request_host,
	read_connect_request,
	get_domain_from_host, 
	CONNECT_RESPONSE,
	BAD_REQUEST,
};
use crate::block::Blocklist;

pub struct NoAdsHttpsProxy {
	addr: SocketAddr,
	x_fwd_addr: SocketAddr,
	db_uri: String,

}

impl NoAdsHttpsProxy {
	pub fn new(addr: SocketAddr, x_fwd_addr: SocketAddr, db_uri: String) -> Self 
	{
		Self {
			addr,
			x_fwd_addr,
			db_uri,
		}
	}

	#[tokio::main]
	pub async fn handle(&self) -> Result<()> {
		let semaphore = Arc::new(Semaphore::new(1000));
		let listener = TcpListener::bind(self.addr).await?;
		let blocklist = Arc::new(Blocklist::new(&self.db_uri));

		if self.addr == self.x_fwd_addr {
			println!("[INFO] NoAdsHttpsProxy is listening on {}", self.addr);
		}
		else {
			println!(
				"[INFO] NoAdsHttpsProxy is listening on {} and forwarding to {}", 
				self.addr, 
				self.x_fwd_addr,
			);
		}


		loop {
    		let permit = match semaphore.clone().acquire_owned().await {
    			Ok(permit) => permit,
    			Err(e) => panic!("NoAdsHttpsProxy::handle(), {:?}", e),
    		};

    		if semaphore.available_permits() <= 100 {
	    		println!("[WARN] Available permits {} (10%)", semaphore.available_permits());
    		} 

			let (mut c_stream, c_addr) = listener.accept().await?;
			let b_list = blocklist.clone();
			let x_fwd_addr = self.x_fwd_addr.clone();

			tokio::spawn(async move {
				let _ = Self::transfer(
					c_stream, 
					c_addr, 
					x_fwd_addr, 
					b_list,
				).await;

				drop(permit);
			});
		}
	}

	async fn transfer(
		mut c_stream: TcpStream, 
		c_addr: SocketAddr, 
		x_fwd_addr: SocketAddr,
		b_list: Arc<Blocklist>,
	) -> Result<()> 
	{
		let request = read_connect_request(&mut c_stream).await?;
		let host = match parse_connect_request_host(&request)? {
			Some(addr) => addr,
			None => return Ok(()),
		};
		let domain = get_domain_from_host(&host)?;

		let _ = match b_list.is_domain_blocked(&domain).await {
			Ok(true) => {
				println!("[INFO] Denied connection to {}", domain);
				c_stream.write_all(BAD_REQUEST).await?;	
				return Ok(());
			},
			Ok(false) => (),
			Err(e) => return Err(e),
		};	

		// Directly connect to the target host
		if c_stream.local_addr()? == x_fwd_addr {
			let mut s_stream = TcpStream::connect(host).await?;
			c_stream.write_all(CONNECT_RESPONSE).await?;
			copy_bidirectional(&mut c_stream, &mut s_stream).await?;

			return Ok(());
		} 

		// Forward the request to the target proxy
		let mut s_stream = TcpStream::connect(x_fwd_addr).await?;
		s_stream.write_all(&request[..]).await?;
		copy_bidirectional(&mut c_stream, &mut s_stream).await?;

		Ok(())
	}
}