use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use redb::Database;
use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Clone)]
pub struct Options<'a> {
    pub user_agent: String,
    pub db: &'a Database,
    pub request_timeout: u64,
    pub concurrent_num: usize,
    pub proxy_server: String,
    pub proxy_user: String,
    pub proxy_pass: String,
    pub follow_redirect: reqwest::redirect::Policy,
    pub wordlist: Vec<String>,
    pub fuzz_url: String,
    pub result_file: String,
    pub custom_matches: Matches,
    pub print_state: bool,
    pub request_retries: usize,
    pub sys: &'a System,
    pub scan_mode: ScanMode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MatchResult {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Matches {
    pub regex: Option<String>,
    pub status_code: HashSet<u16>,
    pub line_num: Option<usize>,
    pub resp_size: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct HttpResp {
    pub url: String,
    pub status: u16,
    pub html: Vec<u8>,
    pub duration: Duration,
}

#[derive(PartialEq, Clone, Debug)]
pub enum ScanMode {
    DEBUG,
    STATS,
    SILENT,
}
impl ToString for ScanMode {
    fn to_string(&self) -> String {
        match self {
            ScanMode::DEBUG => String::from("debug"),
            ScanMode::STATS => String::from("detail"),
            ScanMode::SILENT => String::from("silent"),
        }
    }
}

#[derive(Hash, PartialEq, Debug)]
pub enum Stats {
    IOErr,
    Done,
    Hits,
    DNSErr,
    TimeOut,
    C404,
    C500,
    C200,
    C301,
    C302,
    C403,
    C401,
    C000,
}