use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::Duration;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Clone, Debug)]
pub struct Params {
    pub user_agent: String,
    pub request_timeout: u64,
    pub concurrent_num: usize,
    pub proxy_server: String,
    pub proxy_user: String,
    pub proxy_pass: String,
    pub follow_redirect: usize,
    pub wordlist: HashSet<String>,
    pub wordlist_len: usize,
    pub fuzz_url: String,
    pub result_file: String,
    pub print_state: bool,
    pub request_retries: usize,
    pub script_option: Option<ScriptOpt>,
    pub scan_mode: ScanMode,
    pub no_color: bool,
    pub custom_matches: Option<Matches>,
    pub custom_filters: Option<FilterRules>,
}

#[derive(Clone)]
pub struct Options<'a> {
    pub sys: &'a System,
    pub params: Params,
}

#[derive(Clone, Debug, Default)]
pub struct ScriptOpt {
    pub script_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MatchResult {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Matches {
    pub regex: Option<Regex>,
    pub status_code: Option<HashSet<u16>>,
    pub line_num: Option<usize>,
    pub resp_size: Option<usize>,
    pub and_and_and: bool,
}

#[derive(Debug, Clone)]
pub struct FilterRules {
    pub regex: Option<Regex>,
    pub status_code: Option<HashSet<u16>>,
    pub line_num: Option<HashSet<usize>>,
    pub resp_size: Option<Vec<String>>,
    pub and_and_and: bool,
}

#[derive(Clone, Debug)]
pub struct HttpResp {
    pub url: String,
    pub status: u16,
    pub html: Vec<u8>,
    pub duration: Duration,
    pub remote_addr: Option<SocketAddr>,
}

#[derive(PartialEq, Clone, Debug)]
pub enum ScanMode {
    Debug,
    Stats,
    Silent,
}

impl ToString for ScanMode {
    fn to_string(&self) -> String {
        match self {
            ScanMode::Debug => String::from("debug"),
            ScanMode::Stats => String::from("detail"),
            ScanMode::Silent => String::from("silent"),
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