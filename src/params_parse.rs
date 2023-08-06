use std::collections::HashSet;
use regex::Regex;
use crate::data_type::{FilterRules, Matches};
use crate::G_DEFAULT_MATCHES_STATUS_CODE;

pub fn opt_int_parm(name: &str, matches: &getopts::Matches, default: usize) -> usize {
    if let Some(s) = matches.opt_str(name) {
        if let Ok(u) = s.parse::<usize>() {
            return u;
        }
    }
    default
}

pub fn opt_int_some_parm(name: &str, matches: &getopts::Matches) -> Option<usize> {
    if let Some(s) = matches.opt_str(name) {
        if let Ok(u) = s.parse::<usize>() {
            return Some(u);
        }
    }
    None
}

pub fn opt_usize_split(name: &str, defaults: &str, matches: &getopts::Matches) -> Option<HashSet<usize>> {
    let mut codes: HashSet<usize> = HashSet::new();
    match matches.opt_str(name) {
        Some(s) => {
            let split: Vec<&str> = s.split(',').collect();
            for code in split {
                if let Ok(u) = code.parse::<usize>() {
                    codes.insert(u);
                }
            }
            Some(codes)
        }
        None => {
            if defaults.is_empty() {
                return None
            }
            let split: Vec<&str> = defaults.split(',').collect();
            for code in split {
                if let Ok(u) = code.parse::<usize>() {
                    codes.insert(u);
                }
            }
            Some(codes)
        }
    }
}

pub fn opt_int_split(name: &str, defaults: &str, matches: &getopts::Matches) -> Option<HashSet<u16>> {
    let mut codes: HashSet<u16> = HashSet::new();
    match matches.opt_str(name) {
        Some(s) => {
            let split: Vec<&str> = s.split(',').collect();
            for code in split {
                if let Ok(u) = code.parse::<u16>() {
                    codes.insert(u);
                }
            }
            Some(codes)
        }
        None => {
            if defaults.is_empty() {
                return None
            }
            let split: Vec<&str> = defaults.split(',').collect();
            for code in split {
                if let Ok(u) = code.parse::<u16>() {
                    codes.insert(u);
                }
            }
            codes.insert(200);
            codes.insert(403);
            Some(codes)
        }
    }
}

pub fn opt_vec_split(name: &str, matches: &getopts::Matches) -> Option<Vec<String>> {
    let mut list: Vec<String> = Vec::new();
    match matches.opt_str(name) {
        Some(s) => {
            list = s.split(',').map(|v| { v.trim().to_string() }).collect();
            Some(list)
        }
        None => None
    }
}

pub fn filter_params(matches: &getopts::Matches) -> Result<Option<FilterRules>, String> {
    let filter_resp_regex = match matches.opt_str("fr") {
        Some(r) => {
            match Regex::new(&r) {
                Ok(r) => Some(r),
                Err(e) => {
                    return Err(e.to_string());
                }
            }
        }
        None => None
    };
    let filter_http_status_code = opt_int_split("fc","", matches);
    let filter_resp_line = opt_usize_split("fl", "", matches);

    let filter_resp_size = opt_vec_split("fs", matches);
    // empty
    if filter_resp_regex.is_none() && filter_http_status_code.is_none() &&
        filter_resp_line.is_none() && filter_resp_size.is_none() {
        return Ok(None)
    }
    Ok(Some(FilterRules{
        regex: filter_resp_regex,
        status_code: filter_http_status_code,
        line_num: filter_resp_line,
        resp_size: filter_resp_size,
        and_and_and: matches.opt_present("or-filter").eq(&false),
    }))
}

pub fn match_params(matches: &getopts::Matches) -> Result<Option<Matches>, String> {
    // match
    let match_regex = match matches.opt_str("mr") {
        Some(r) => {
            match Regex::new(&r) {
                Ok(r) => Some(r),
                Err(e) => {
                    return Err(e.to_string());
                }
            }
        }
        None => None
    };

    let match_http_status_code = opt_int_split("mc", G_DEFAULT_MATCHES_STATUS_CODE, matches);
    let match_resp_size = opt_int_some_parm("ms", matches);
    let match_resp_line = opt_int_some_parm("ml", matches);

    // empty
    if match_regex.is_none() && match_http_status_code.is_none() &&
        match_resp_size.is_none() && match_resp_line.is_none() {
        return Ok(None)
    }

    Ok(Some(Matches {
        regex: match_regex,
        status_code: match_http_status_code,
        line_num: match_resp_line,
        resp_size: match_resp_size,
        and_and_and: matches.opt_present("or-match").eq(&false),
    }))
}
