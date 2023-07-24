mod common;
mod scanner;
mod data_type;
mod data_handler;
mod error;

use getopts::Options;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{HashSet, LinkedList};
use std::{env, mem};
use std::fs;
use std::fs::File;
use std::hash::Hash;
use std::io::{BufRead, BufReader, Error, IsTerminal, Read};
use std::process;
use std::sync::atomic::{AtomicBool};
use std::sync::{Arc, RwLock};
use std::io::Write;
use console::{style, StyledObject};
use log::LevelFilter;
use rand::Rng;
use crate::common::{adjust_ulimit_size, bytes_to_gb, bytes_to_mb, COMMON_USER_AGENTS, DEFAULT_DNS_SERVERS, file_exists, opt_int_parm, opt_int_some_parm};
use crate::data_type::*;
use crate::scanner::Scanner;
use chashmap::CHashMap;
use redb::{Database, ReadableTable, TableDefinition};
use sysinfo::{System, SystemExt};


#[derive(Debug)]
struct Queue<T> {
    queue: LinkedList<T>,
}

impl<T> Queue<T> {
    fn new() -> Self {
        Queue {
            queue: LinkedList::new(),
        }
    }

    fn enqueue(&mut self, item: T) {
        self.queue.push_back(item);
    }
    fn dequeue(&mut self) -> Option<T> {
        self.queue.pop_front()
    }
}

lazy_static! {
    static ref G_RESPONSE: Arc<RwLock<Queue<HttpResp>>> = {
        let queue = Queue::new();
        let rwlock = RwLock::new(queue);
        Arc::new(rwlock)
    };
    pub static ref G_LOOP_BREAK: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
    pub static ref G_SCAN_MODE: Arc<ScanMode> = Arc::new(ScanMode::DEBUG);

    pub static ref G_STATS: Arc<CHashMap<Stats, u32>> = {
        let stats = CHashMap::new();
        stats.insert(Stats::IOErr, 0);
        stats.insert(Stats::Done, 0);
        stats.insert(Stats::Hits, 0);
        stats.insert(Stats::TimeOut, 0);
        stats.insert(Stats::DNSErr, 0);
        stats.insert(Stats::C404, 0);
        stats.insert(Stats::C200, 0);
        stats.insert(Stats::C500, 0);
        stats.insert(Stats::C403, 0);
        stats.insert(Stats::C401, 0);
        stats.insert(Stats::C302, 0);
        stats.insert(Stats::C301, 0);
        stats.insert(Stats::C000, 0);
        Arc::new(stats)
    };
}

const G_PATH_TABLE: TableDefinition<&str, u64> = TableDefinition::new("hits_uri");
const G_DEFAULT_FILE_DESC_LIMIT: u64 = 65535;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn print_start_info(option: &data_type::Options) {
    let mem_size = mem::size_of_val(&*option.wordlist);
    println!("wordlist: {}/lines, mem size: {:.2}/Mb", option.wordlist.len(), bytes_to_mb(mem_size as u64));
    println!("concurrents: {}",  option.concurrent_num);
    println!("retries: {}", option.request_retries);
    println!("request timeout: {}/s", option.request_timeout);
    println!("User-agent: {}", option.user_agent);
    println!("DNS Servers: {}", DEFAULT_DNS_SERVERS.len());
    println!("total memory: {:.2}/Gb", bytes_to_gb(option.sys.total_memory()));
    println!("total swap: {:.2}/Mb", bytes_to_mb(option.sys.total_swap()));
    println!("mode: {}", option.scan_mode.to_string());
    println!();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = Box::new(File::create("kenshi.log").expect("Can't create file"));
    env_logger::Builder::new()
        .target(env_logger::Target::Pipe(target))
        .filter(None, LevelFilter::Info)
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {}:{}] {}",
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optopt("u", "url", "required. Test url", "URL");
    opts.optopt("w", "wordlist", "required. fuzz wordlist", "FILE");
    opts.optopt("o", "output", "Output result", "FILE");

    // match option
    opts.optopt("", "mc", r#"Match HTTP status codes, or "all" for everything. (default: 200,403)"#, "");
    opts.optopt("", "mr", "Match regexp", "regexp");
    opts.optopt("", "ms", r#"Match HTTP response size"#, "length");
    opts.optopt("", "ml", r#"Match amount of lines in response"#, "int");
    opts.optopt("", "mt", "Match how many milliseconds to the first response byte, either greater or less than. EG: >100 or <100", "");
    // filter option
    opts.optopt("", "fc", "Filter HTTP status codes from response. Comma separated list of codes and ranges", "regexp");
    opts.optopt("", "fl", "Filter by amount of lines in response. Comma separated list of line counts and ranges", "");
    opts.optopt("", "fmode", r#"Filter set operator. Either of: and, or (default: or)"#, "");
    opts.optopt("", "fr", r#"Filter regexp"#, "");
    opts.optopt("", "fs", r#"Filter HTTP response size. Comma separated list of sizes and ranges"#, "");

    opts.optopt("", "rt", "request timeout", "Int");
    opts.optopt("c", "parallel", "Number of parallel requests", "1000");
    opts.optflag("f", "follow-redirect", "enable redirect 301/302, default is false,");
    opts.optopt("r", "retrie", "Number of failed retry requests", "1");
    opts.optopt("x", "proxy", "proxy request, http/https/socks5", "socks5://1.1.1.1:1080");
    opts.optopt("U", "auth", "proxy auth, if required", "username:password");
    opts.optflag("", "clear", "cache Clear");
    // mode
    opts.optflag("", "debug", "More detailed logging mode");
    opts.optflag("", "silent", "silent mode");
    opts.optflag("v", "stats", "Display detailed scanning status");
    opts.optopt("", "dns-list", "Specify a list of name servers", "Url or File");
    //opts.optopt("p", "port", "binding port", "PORT");
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            println!("{}", f.to_string());
            return Ok(());
        }
    };

    if args.len() == 1 || matches.opt_present("h") {
        print_usage(&program, opts);
        return Ok(());
    }

    // init env
    if fs::create_dir_all("data").is_err() {
        log::error!("sled directory create failed.");
        process::exit(1);
    }

    let db = Database::create("cache")?;

    if matches.opt_present("clear") {
        println!("OK.");
        process::exit(1);
    }

    let result_path = match matches.opt_str("o") {
        Some(f) => {
            if file_exists(&f) {
                println!("{} exists, please note.", &f);
                process::exit(1);
            }
            f
        }
        None => {
            "output.txt".to_string()
        }
    };

    let fuzz_url = match matches.opt_str("u") {
        Some(f) => {
            f
        }
        None => {
            println!("fuzz url is empty.");
            process::exit(1);
        }
    };

    let match_regex = match matches.opt_str("mr") {
        Some(r) => {
            match Regex::new(&r) {
                Ok(_) => Some(r),
                Err(e) => {
                    return Err(Box::try_from(e).unwrap());
                },
            }
        }
        None => None
    };

    // proxy support
    let mut proxy_server= String::new();
    let mut proxy_user= String::new();
    let mut proxy_pass= String::new();
    if let Some(s) = matches.opt_str("p") {
        proxy_server = s;
        if let Some(s) = matches.opt_str("U") {
            let auth: Vec<&str> = s.splitn(2, ':').collect();
            if auth.len() != 2 {
                return Err(Box::try_from("Failed to format proxy authentication information".to_string()).unwrap())
            }
            (proxy_user, proxy_pass) = (auth.first().unwrap().to_string(), auth.get(1).unwrap().to_string());
        }
    }

    let request_retries = opt_int_parm("r", &matches, 1);
    let debug_mode = matches.opt_present("debug");
    let mut concurrent_num = opt_int_parm("c", &matches, 100);
    let ulimit = concurrent_num as u64 * 2;
    if ulimit > G_DEFAULT_FILE_DESC_LIMIT {
        let _ = adjust_ulimit_size(G_DEFAULT_FILE_DESC_LIMIT) as usize;
    }

    let request_timeout_sec = opt_int_parm("rt", &matches, 10) as u64;
    //
    let mut codes: HashSet<u16> = HashSet::new();
    let match_http_status_code = match matches.opt_str("mc") {
        Some(s) => {
            let split: Vec<&str> = s.split(',').collect();

            for code in split {
                if let Ok(u) = s.parse::<u16>() {
                    codes.insert(u);

                }
            }
            codes
        }
        None => {
            codes.insert(200);
            codes.insert(403);
            codes
        }
    };


    let match_resp_size = opt_int_some_parm("ms", &matches);
    let match_resp_line = opt_int_some_parm("ml", &matches);

    let custom_matches = Matches {
        regex: match_regex,
        status_code: match_http_status_code,
        line_num: match_resp_line,
        resp_size: match_resp_size,
    };

    let mut sys = System::new_all();
    sys.refresh_all();

    // wordlist load


    let wordlist_path = match matches.opt_str("w") {
        Some(path) => {
            if !file_exists(&path) {
                println!("{} don't exist.", &path);
                process::exit(1);
            }
            path
        }
        None => {
            println!("wordlist is Empty.");
            process::exit(1);
        }
    };

    print!("Load... ");
    std::io::stdout().flush()?;
    let mut wordlist: HashSet<String> = HashSet::new();
    let is_tty = std::io::stdin().is_terminal();
    if is_tty {
            //Stream processing seems more appropriate
            let file = File::open(wordlist_path).expect("Failed to open file");
            let reader = BufReader::new(&file);
            for payload in reader.lines() {
                wordlist.insert(payload.unwrap());
            }
    }else {
        for line in std::io::stdin().lines().flatten() {
                wordlist.insert(line);
        }
    }
    let wordlist: Vec<String> = wordlist.iter().cloned().collect();
    println!("DONE");
    if concurrent_num > wordlist.len() {
        concurrent_num = wordlist.len() / 2;
        if concurrent_num == 0 {
            concurrent_num = 1
        }
    }

    let mut scan_mode = ScanMode::DEBUG;

    if matches.opt_present("stats") {
        scan_mode = ScanMode::STATS
    }

    if matches.opt_present("silent") {
        scan_mode = ScanMode::SILENT
    }

    let mut print_state = false;
    if scan_mode == ScanMode::DEBUG || scan_mode == ScanMode::STATS {
        print_state = true
    }
    let mut rng = rand::thread_rng();
    let ua_str = COMMON_USER_AGENTS[rng.gen_range(0..COMMON_USER_AGENTS.len() - 1)];

    let options = data_type::Options {
        user_agent: ua_str.to_owned(),
        request_timeout: request_timeout_sec,
        db: &db,
        sys: &sys,
        result_file: result_path,
        wordlist: wordlist.clone(),
        fuzz_url,
        print_state,
        concurrent_num,
        custom_matches,
        request_retries,
        proxy_server,
        proxy_user: proxy_user.to_owned(),
        proxy_pass: proxy_pass.to_owned(),
        scan_mode
    };
    print_state.then(||{
        print_start_info(&options)
    });



//
    let scan = Scanner::new(&options);
    scan.start().await;
    Ok(())
}

//test
