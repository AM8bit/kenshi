use std::fs;
use crate::data_type::{FilterRules, Matches};

pub const COMMON_USER_AGENTS: [&str; 4] = [
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

pub fn file_exists(filename: &str) -> bool {
    fs::metadata(filename).is_ok()
}

pub fn is_match(raw_str: &str, matches: &Matches) -> bool {
    let html = raw_str;
    let mut is_match = true;
    let mut or_or_or = 0u8;
    // status code match
    /*
    if matches.status_code.contains(&resp.status) {
        is_match = true
    } else {
        return None
    }
     */

    // regex match
    if let Some(regex) = &matches.regex {
        let n = regex.is_match(&html);
        is_match = n && is_match;
        n.then(||{ or_or_or += 1});
    }
    // line match
    if let Some(n) = &matches.line_num {
        let n = html.lines().count().eq(n);
        is_match = n && is_match;
        n.then(||{ or_or_or += 1});
    }

    // response size
    if let Some(n) = &matches.resp_size {
        let n = html.len().eq(n);
        is_match = n && is_match;
        n.then(||{ or_or_or += 1});
    }

    if !matches.and_and_and && or_or_or.gt(&0) {
        is_match = true
    }
    if is_match {
        return true
    }
    false
}

pub fn is_filter(raw_str: &str, matches: &FilterRules) -> bool {
    let html = raw_str;
    let mut is_filter = true;
    let mut or_or_or = 0u8;
    // status code match
    /*
    if matches.status_code.contains(&resp.status) {
        is_match = true
    } else {
        return None
    }
     */

    // regex match
    if let Some(regex) = &matches.regex {
        let n = regex.is_match(&html);
        is_filter = n && is_filter;
        n.then(||{ or_or_or += 1});
    }
    // line match
    if let Some(fls) = &matches.line_num {
        let ret = fls.contains(&html.lines().count());
        is_filter = ret && is_filter;
        ret.then(||{ or_or_or += 1});
    }

    // response size
    if let Some(list) = &matches.resp_size {
        let resp_size = html.len();
        let mut ret = false;
        for v in list.iter() {
            let range_str: Vec<&str> = v.split('-').collect();
            if range_str.len().eq(&2) {
                let mut start = 0;
                let mut end = 0;
                if let Ok(u) = range_str.first().unwrap().parse::<usize>() {
                    start = u
                }else {
                    continue
                }
                if let Ok(u) = range_str.last().unwrap().parse::<usize>() {
                    end = u
                }else {
                    continue
                }
                ret = resp_size >= start && resp_size <= end;
                if ret {
                    break
                }
            }else {
                let size_str: String = v.chars()
                    .filter(|c| c.is_numeric())
                    .collect();
                let mut filter_size = 0usize;
                if let Ok(u) = size_str.parse::<usize>() {
                    filter_size = u
                }else {
                    continue
                }
                if v.contains('>') {
                    ret = resp_size.gt(&filter_size);
                }else if v.contains('<') {
                    ret = resp_size.lt(&filter_size);
                }else {
                    ret = resp_size.eq(&filter_size);
                }
                if ret {
                    break
                }
            }

        }
        is_filter = ret && is_filter;
        ret.then(||{ or_or_or += 1});
    }

    if !matches.and_and_and && or_or_or.gt(&0) {
        is_filter = true
    }
    is_filter
}

/*
pub fn is_filter(resp: &HttpResp, matches: &FilterRules) -> bool {
    let html = &resp.html;
    let html = String::from_utf8(html.to_vec());
    if html.is_err() {
        return true;
    }

    let html = html.unwrap();
    let mut is_filter = true;
    // status code match
    /*
    if matches.status_code.contains(&resp.status) {
        is_match = true
    } else {
        return None
    }
     */
    // regex match
    if let Some(regex) = &matches.regex {
        is_filter = regex.is_match(&html) && is_filter;
    }
    // line match
    if let Some(n) = &matches.line_num {
        is_filter = html.lines().count().eq(n) && is_filter
    }

    // response size
    if let Some(n) = &matches.resp_size {
        is_filter = html.len().eq(n) && is_filter
    }

    is_filter
}

 */

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
