use std::fs::File;
use std::collections::HashMap;
use std::sync::Arc;
use std::io::{ Error, Result, BufRead, BufReader };
use tokio::sync::{ Mutex, RwLock };
use rusqlite::{ params, Connection };

pub fn init_blocklist(path: &str) {
	let conn = match Connection::open(path) {
		Ok(database_connection) => database_connection,
		Err(e) => panic!("{:?}", e),
	};

	// Delete the old "blocklist" table
	if let Err(e) = conn.execute("DROP TABLE IF EXISTS blocklist", ()) {
		panic!("An error occurred when tried to drop the \"blocklist\" {e}");
	}

	// Create a table and name it "blocklist"
	if let Err(e) = conn.execute(
		"CREATE TABLE blocklist (
			id INTEGER PRIMARY KEY,
			domain TEXT NOT NULL UNIQUE
		)",
		(),
	) 
	{
		panic!("An error occured when tried to create blocklist table {:?}", e);
	}

	if let Err(e) = conn.execute(
		"CREATE INDEX IF NOT EXISTS idx_domain ON blocklist(domain)", 
		[],
	)
	{
		panic!("An error occured when tried to index the blocklist table, {e}");
	};

	conn.close();
}

pub fn add_blocklist(db_uri: &str, hagezi_list: &str) {
	let mut conn = match Connection::open(db_uri) {
		Ok(database_connection) => database_connection,
		Err(e) => panic!("add_blocklist(), {:?}", e),
	};

	let domain_txt = match File::open(hagezi_list) {
		Ok(txt) => txt,
		Err(e) => panic!("add_blocklist(), {:?}", e),
	};

    let reader = BufReader::new(domain_txt);
    
    let tx = match conn.transaction() {
    	Ok(tx) => tx,
    	Err(e) => panic!("add_blocklist(), {:?}", e),
    };
   
   	{
    	let mut stmt = match tx.prepare(
    		"INSERT INTO blocklist (domain) VALUES (?1)"
    	) 
    	{
    		Ok(stmt) => stmt,
    		Err(e) => panic!("add_blocklist(), {:?}", e),
    	};
    
    	// Iterate through each line in the file
    	for line in reader.lines() {
    		let domain = match line {
    			Ok(ln) => ln.trim().to_string(),
    			Err(e) => panic!("add_blocklist(), {:?}", e),
    		};

    		if domain.is_empty() {
    			continue;
    		}

        	if let Err(e) = stmt.execute(params![domain]) {
        		panic!("add_blocklist(), {:?}", e);
        	}
    	}
	}
    
    // Commit the transaction
    if let Err(e) = tx.commit() {
    	panic!("add_blocklist(), {:?}", e);
    }
}

pub struct Blocklist {
	conn_lock: Mutex<Connection>,
}

impl Blocklist {
	pub fn new(db_uri: &str) -> Self {
		let conn_lock = match Connection::open(db_uri) {
			Ok(db_conn) => Mutex::new(db_conn),
			Err(e) => panic!("Blocklist::new(), {:?}", e),
		};

		return Self { 
			conn_lock,
		}
	}

	pub async fn is_domain_blocked(&self, domain: &str) -> Result<bool> {
		let conn = self.conn_lock.lock().await;

		let query = "SELECT domain FROM blocklist WHERE domain = (?1) LIMIT 1";
		let mut stmt = match conn.prepare(query) {
			Ok(stmt) => stmt,
			Err(e) => {
				let details = format!("Blocklist::is_domain_blocked(), {:?}", e);
				return Err(Error::other(details));
			}
		};

	   	match stmt.exists([domain]) {
	   		Ok(exists) => return Ok(exists),
	   		Err(e) => {
	   			let details = format!("Blocklist::is_domain_blocked(), {:?}", e);	
	   			return Err(Error::other(details));
	   		},
	   	};
	}
}