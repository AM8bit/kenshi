use std::{fs};
use regex::Regex;
use crate::data_type::{Matches, MatchResult, HttpResp};

pub const COMMON_USER_AGENTS : [&str; 4] = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/111.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Linux; Android 10; Redmi Note 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/111.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 11; vivo 1906) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/111.0.0.0 Mobile Safari/537.36"
];

pub const DEFAULT_DNS_SERVERS: [&str; 39] = [
    "8.8.4.4",
    "1.1.1.1",
    "8.8.8.8",
    "209.244.0.3",
    "223.5.5.5",
    "45.11.45.11",
    "209.244.0.4",
    "223.6.6.6",
    "156.154.70.1",
    "156.154.71.1",
    "185.222.222.222",
    "74.82.42.42",
    "64.6.64.6",
    "77.88.8.1",
    "199.85.126.10",
    "199.85.127.10",
    "8.26.56.26",
    "84.200.70.40",
    "84.200.69.80",
    "149.112.112.112",
    "5.2.75.75",
    "208.67.220.220",
    "208.67.222.222",
    "1.0.0.1",
    "9.9.9.9",
    "64.6.65.6",
    "8.20.247.20",
    "80.80.81.81",
    "80.80.80.80",
    "77.88.8.8",
    "195.46.39.39",
    "195.46.39.40",
    "210.2.4.8",
    "1.2.4.8",
    "117.50.22.22",
    "180.76.76.76",
    "89.233.43.71",
    "119.29.29.29",
    "119.28.28.28",
];

pub async fn dns_test() {
    // test hosts 53 port open
}

pub fn file_exists(filename: &str) -> bool {
    fs::metadata(filename).is_ok()
}

pub fn  opt_int_parm(name: &str, matches: &getopts::Matches, default: usize)-> usize {
    if let Some(s) = matches.opt_str(name) { 
        if let Ok(u) = s.parse::<usize>() {
            return u
        }
    }
    default
}

pub fn  opt_int_some_parm(name: &str, matches: &getopts::Matches)-> Option<usize> {
    if let Some(s) = matches.opt_str(name) {
        if let Ok(u) = s.parse::<usize>() {
            return Some(u)
        }
    }
    None
}

pub fn  opt_int_some_parm2<T>(name: &str, matches: &getopts::Matches)-> Option<T> {
    if let Some(s) = matches.opt_str(name) {
        if let Ok(u) = s.parse::<usize>() {
            let u = u as u8;
           // return Some(u)
        }
    }
    None
}


pub fn is_match(resp: &HttpResp, matches: &Matches) -> Option<MatchResult> {
    let html = &resp.html;
    let html = String::from_utf8(html.to_vec());
    if html.is_err() {
        return None
    }
    let html = html.unwrap();
    let mut is_match = false;
    // status code match
    if matches.status_code.contains(&resp.status) {
        is_match = true
    } else {
        return None
    }
    // regex match
    if let Some(regex) = &matches.regex {
        let re = Regex::new(regex).unwrap();
        is_match = re.is_match(&html) && is_match;
    }
    // line match
    if let Some(n) = &matches.line_num {
        is_match = html.lines().count().eq(n) && is_match
    }

    // response size
    if let Some(n) = &matches.resp_size {
        is_match = html.len().eq(n) && is_match
    }

    if is_match {
        return Some(MatchResult{
            url: resp.url.to_string()
        })
    }
    None
}

pub fn bytes_to_gb(bytes: u64) -> f64 {
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    gb
}

pub fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / (1024f64 * 1024f64)
}
/*
pub struct Cache {
    db: Database,
}

impl Cache {
    pub fn new(db: Database)-> Self {
        Self {
            db
        }
    }

    pub fn cache_insert(&self, table: TableDefinition<&str, u64>)-> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(table)?;
            table.insert("my_key", &123)?;
        }
        write_txn.commit()?;
        Ok(())
    }
    
    pub fn delete_table(&self, table: TableDefinition<&str, u64>)-> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            write_txn.delete_table(table)?;
        }
        write_txn.commit()?;
        Ok(())
    }
    
}
*/


//from RustScan
#[cfg(unix)]
pub fn adjust_ulimit_size(ulimit: u64) -> u64 {
    use rlimit::Resource;

    if Resource::NOFILE.set(ulimit, ulimit).is_ok() {
        log::info!("Automatically increasing ulimit value to {}.", ulimit);
    } else {
        log::warn!("Failed to set ulimit value.");
    }

    let (soft, _) = Resource::NOFILE.get().unwrap();
    soft
}
