#![allow(warnings)] 
mod http;
mod block;
mod config;
mod proxy;

use std::env;
use std::thread;
use crate::proxy::*;
use crate::config::*;
use crate::block::*;

fn read_command() -> (String, String) {
   let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: noadproxy [init|run] [<path-to-config-file>]");
        panic!("NOADPROXY: missing mode or path");
    }

    let mode = args[1].clone();
    let path = args[2].clone();

    if mode != "init" && mode != "run" {
        eprintln!("Usage: noadproxy [init|run] <path-to-config-file>");
        panic!("NOADPROXY: unsupported mode");
    }

    return (mode, path);
}

fn startup() {
    let (mode, path) = read_command();
    let configs = parse_configs(&path);

    if mode == "init" {
        println!("Create a new SQLite database on {}", path);
        init_blocklist(&configs.db_uri);
        println!("Empty database created on {}", path);
        println!("Popluate the 'blocklist' table");
        add_blocklist(&configs.db_uri, &configs.blkls);
        println!("Done");

        return;
    }

    // Run the filter proxy
    tracing_subscriber::fmt().with_target(false).init();    

    let no_ads_https_proxy = NoAdsHttpsProxy::new(
        configs.addr.clone(),
        configs.x_fwd_addr.clone(),
        configs.db_uri.clone(),
    );

    let handle = thread::spawn(move || {
        let _ = no_ads_https_proxy.handle();
    });

    let _ = handle.join();
}

fn main() {
    startup();
}
