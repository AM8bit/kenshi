use std::{env};
use std::collections::{HashSet, LinkedList};
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::io::Write;
use std::process::exit;
use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicBool;

use chashmap::CHashMap;
use console::Term;
use getopts::Options;
use is_terminal::IsTerminal;
use lazy_static::lazy_static;
use log::{LevelFilter};
use rand::{Rng};
use sysinfo::{System, SystemExt};

use crate::common::*;
#[cfg(unix)]
use crate::common::adjust_ulimit_size;
use crate::data_type::*;
use crate::params_parse::{filter_params, match_params, opt_int_parm};
use crate::scanner::Scanner;

mod common;
mod scanner;
mod data_type;
mod data_handler;
mod error;
mod tests;
mod params_parse;
mod rawhttp;
mod dns_preheat;

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
    pub static ref G_SCAN_MODE: Arc<ScanMode> = Arc::new(ScanMode::Debug);

    pub static ref G_STATS: Arc<CHashMap<Stats, u32>> = {
        let stats = CHashMap::new();
        stats.insert(Stats::IOErr, 0);
        stats.insert(Stats::Done, 0);
        stats.insert(Stats::Hits, 0);
        stats.insert(Stats::TimeOut, 0);
        stats.insert(Stats::DNSErr, 0);
        stats.insert(Stats::C404, 0);
        stats.insert(Stats::C200, 0);
        stats.insert(Stats::C502, 0);
        stats.insert(Stats::C500, 0);
        stats.insert(Stats::C403, 0);
        stats.insert(Stats::C401, 0);
        stats.insert(Stats::C302, 0);
        stats.insert(Stats::C301, 0);
        stats.insert(Stats::C000, 0);
        Arc::new(stats)
    };
}

const G_DEFAULT_FILE_DESC_LIMIT: u64 = 65535;
const G_DEFAULT_CONCURRENT_NUM: u32 = 500;
const G_DEFAULT_MATCHES_STATUS_CODE: &str = "200,301,403,401,500";
const G_DEFAULT_LOGFILE: &str = "kenshi.log";
pub const VERSION: &str = "v0.1.3";

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    println!("{}", opts.usage(&brief));
}

fn print_start_info(option: &data_type::Options) {
    println!("wordlist: {}/lines", option.params.wordlist_len);
    println!("concurrent: {}", option.params.concurrent_num);
    println!("retries: {}", option.params.request_retries);
    println!("request timeout: {}/s", option.params.request_timeout);
    println!("user-agent: {}", option.params.user_agent);
    println!("dns servers: {}", DEFAULT_DNS_SERVERS.len());
    println!("dns try: {}", option.params.dns_try);
    println!("memory: {:.2}/Gb", bytes_to_gb(option.sys.total_memory()));
    println!("swap: {:.2}/Mb", bytes_to_mb(option.sys.total_swap()));
    println!("mode: {}", option.params.scan_mode.to_string());
    if option.params.scan_mode == ScanMode::Debug {
        println!("logfile: {G_DEFAULT_LOGFILE}");
    }
    println!();
}

pub fn parse_args(args: &[String]) -> Result<Params, String> {
    let program = args[0].clone();
    let mut opts = Options::new();
    // basic
    opts.optopt("u", "url", "required. Test url", "<url>");
    opts.optopt("w", "wordlist", "required. Wordlist file path. eg. '/path/to/wordlist'", "<file>");
    opts.optopt("o", "output", "Output result", "<file>");

    // match option
    opts.optflag("", "or-match", r#"Any one of these hits will do. (default: and)"#);
    opts.optopt("", "mc", "Match HTTP status codes, or \"all\" for everything.", &format!("<{G_DEFAULT_MATCHES_STATUS_CODE}>"));
    opts.optopt("", "mr", "Match regexp", "<regexp>");
    opts.optopt("", "ms", r#"Match HTTP response size"#, "<int>");
    opts.optopt("", "ml", r#"Match amount of lines in response"#, "<int>");
    //opts.optopt("", "mt", "Match how many milliseconds to the first response byte, either greater or less than. EG: >100 or <100", "");

    // filter option
    opts.optflag("", "or-filter", r#"Any one of these hits will do. (default: and)"#);
    opts.optopt("", "fc", "Filter HTTP status codes from response. Comma separated list of codes and ranges", "<int,...>");
    opts.optopt("", "fl", "Filter by amount of lines in response. Comma separated list of line counts and ranges. eg. --fl 123,1234 ", "<int,...>");
    opts.optopt("", "fr", r#"Filter regexp"#, "<regexp>");
    opts.optopt("", "fs", r#"Filter HTTP response size. Comma separated list of sizes and ranges. eg. --fs "<100,>1000,10-50,1234""#, "<rules...>");

    // scan
    opts.optopt("", "rt", "Request timeout seconds", "<int>");
    opts.optopt("c", "concurrent", &format!("Number of concurrent requests. default: {G_DEFAULT_CONCURRENT_NUM}"), "<int>");
    opts.optopt("", "follow-redirect", "enable redirect 301/302. disabled by default", "<int>");
    opts.optopt("r", "retries", "Number of failed retry requests", "<int>");
    opts.optflag("", "dns-try", "Try multiple sets of nameservers to mitigate dns resolution failures");

    // http option
    /*
    opts.optopt("", "http1", "", "");
    opts.optopt("", "cookie", "", "1");
    opts.optopt("", "raw", "", "1");
    opts.optflag("", "test-raw", r#"debug raw request"#);
    opts.optopt("r", "valid-cert", "Only valid certificate targets will be tested", "1");
    opts.optopt("", "no-color", "I like black and white.", "socks5://1.1.1.1:1080");
     */

    opts.optopt("x", "proxy", "proxy request, http/https/socks5", "<socks5://1.1.1.1:1080>");
    opts.optopt("U", "auth", "proxy auth, if required", "<username:password>");
    //opts.optflag("", "clear", "cache Clear");
    // dirsearch
    opts.optflag("D", "", "Replace wordlist %EXT% keywords with extension. Used in conjunction with -e flag. (default: false)");
    opts.optopt("e", "ext", "Comma separated list of extensions. Extends FUZZ keyword.", "");
    opts.optopt("s", "script", "lua script(This is an experimental feature)", "");
    // mode
    //opts.optopt("m", "mode", "Multi-wordlist operation mode. Available modes: clusterbomb, pitchfork, sniper (default: clusterbomb)", "");
    opts.optflag("", "silent", "silent mode");
    opts.optflag("v", "stats", "Display detailed scanning status");
    opts.optflag("", "vv", "show version");
    //opts.optopt("", "dns-list", "Specify a list of name servers", "Url or File");
    //opts.optopt("p", "port", "binding port", "PORT");
    /*
        -mode
        -request            File containing the raw http request
        -request-proto      Protocol to use along with raw request (default: https)
        -w                  Wordlist file path and (optional) keyword separated by colon. eg. '/path/to/wordlist:KEYWORD'
     */
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            print_usage(&program, opts);
            return Err(f.to_string());
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        exit(1);
    }

    if matches.opt_present("vv") {
        println!("{VERSION}");
        exit(0);
    }

    // output file
    let result_path = match matches.opt_str("o") {
        Some(result_path) => {
            if file_exists(&result_path) {
                return Err(format!("{} exists, please note.", &result_path));
            }
            Some(result_path)
        }
        None => None
    };


    let fuzz_url = match matches.opt_str("u") {
        Some(f) => {
            if !f.contains("FUZZ") {
                return Err(r#"not found "FUZZ" str."#.to_string());
            }
            f
        }
        None => {
            return Err("fuzz url is empty.".to_string());
        }
    };


    // proxy support
    let mut proxy_server = String::new();
    let mut proxy_user = String::new();
    let mut proxy_pass = String::new();
    if let Some(s) = matches.opt_str("x") {
        proxy_server = s;
        if let Some(s) = matches.opt_str("U") {
            let auth: Vec<&str> = s.splitn(2, ':').collect();
            if auth.len() != 2 {
                return Err("Failed to format proxy authentication information".to_string());
            }
            (proxy_user, proxy_pass) = (auth.first().unwrap().to_string(), auth.get(1).unwrap().to_string());
        }
    }

    let request_retries = opt_int_parm("r", &matches, 1);
    let mut concurrent_num = opt_int_parm("c", &matches, G_DEFAULT_CONCURRENT_NUM as usize);
    let ulimit = concurrent_num as u64 * 2;

    #[cfg(unix)]
    if ulimit > G_DEFAULT_FILE_DESC_LIMIT {
        let _ = adjust_ulimit_size(G_DEFAULT_FILE_DESC_LIMIT) as usize;
    }

    let request_timeout_sec = opt_int_parm("rt", &matches, 10) as u64;
    let follow_redirect_num = opt_int_parm("follow-redirect", &matches, 0);

    let custom_filter = match filter_params(&matches) {
        Ok(r) => r,
        Err(e) => return Err(e)
    };
    let custom_matches = match match_params(&matches) {
        Ok(r) => r,
        Err(e) => return Err(e)
    };

    // wordlist load
    let term = Term::stdout();
    print!("Load... ");
    let _ = std::io::stdout().flush();
    let mut wordlist: HashSet<String> = HashSet::new();
    let is_tty = std::io::stdin().is_terminal();
    if is_tty {
        if let Some(path) = matches.opt_str("w") {
            if !file_exists(&path) {
                let _ = term.clear_line();
                return Err(format!(r#"wordlist "{path}" non-existent."#));
            }
            //Stream processing seems more appropriate
            let file = File::open(path).expect("Failed to open file");
            let reader = BufReader::new(&file);
            for payload in reader.lines() {
                wordlist.insert(payload.unwrap());
            }
        }else {
            let _ = term.clear_line();
            return Err("missing -w param, -h Get Help.".to_owned())
        }
    } else {
        for line in std::io::stdin().lines().flatten() {
            wordlist.insert(line);
        }
    }

    // dirsearch ext replace
    if matches.opt_present("D") {
        let mut ext_s: Vec<String> = vec![];
        if let Some(s) = matches.opt_str("e") {
            ext_s = s.split(',').map(|s| s.to_string()).collect();
        }
        if ext_s.is_empty() {
            let _ = term.clear_line();
            return Err("extensions invalid. for example:-e php,db,conf,bak".to_string());
        }

        let mut new_wordlist: HashSet<String> = HashSet::new();
        for line in wordlist.iter() {
            if line.contains("%EXT%") {
                for s in ext_s.iter() {
                    new_wordlist.insert(line.replace("%EXT%", s));
                }
                continue;
            }
            new_wordlist.insert(line.to_string());
        }
        wordlist = new_wordlist;
    }
    println!("DONE");

    // Dealing with some strange situations
    if concurrent_num > wordlist.len() {
        concurrent_num = wordlist.len() / 2;
        if concurrent_num == 0 {
            concurrent_num = 1
        }
    }

    // enable script
    let script = match matches.opt_str("s") {
        Some(s) => {
            let path = format!("script/{s}.kse");
            if !file_exists(&path) {
                return Err(format!("[script] {path} no exist."))
            }
            Some(ScriptOpt{
                script_path: path,
            })
        }
        _ => None
    };

    // other
    let mut no_color = false;
    /*
    if matches.opt_present("no-color") {
        no_color = true;
    }
     */
    // scan state
    let mut scan_mode = ScanMode::Debug;

    if matches.opt_present("stats") {
        scan_mode = ScanMode::Stats
    }

    if matches.opt_present("silent") {
        scan_mode = ScanMode::Silent
    }

    let mut print_state = false;
    if scan_mode == ScanMode::Debug || scan_mode == ScanMode::Stats {
        print_state = true
    }

    // dns mitigate
    let dns_try = matches.opt_present("dns-try");

    // User-agent random choose
    let mut rng = rand::thread_rng();
    let ua_str = COMMON_USER_AGENTS[rng.gen_range(0..COMMON_USER_AGENTS.len() - 1)];

    Ok(Params {
        user_agent: ua_str.to_owned(),
        request_timeout: request_timeout_sec,
        result_file: result_path,
        wordlist_len: wordlist.len(),
        wordlist,
        fuzz_url,
        print_state,
        concurrent_num,
        custom_matches,
        request_retries,
        proxy_server,
        dns_try,
        proxy_user: proxy_user.to_owned(),
        proxy_pass: proxy_pass.to_owned(),
        script_option: script,
        scan_mode,
        follow_redirect: follow_redirect_num,
        custom_filters: custom_filter,
        no_color,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = Box::new(File::create(G_DEFAULT_LOGFILE).expect("Can't create file"));
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

    if fs::create_dir_all("data").is_err() {
        log::error!("cache directory create failed.");
        exit(1);
    }

    let mut sys = System::new_all();
    sys.refresh_all();

    let args: Vec<String> = env::args().collect();
    let params = parse_args(&args);
    if let Err(e) = params {
        eprintln!("{e}");
        exit(1)
    }
    let params = params.unwrap();
    let options = data_type::Options {
        sys: &sys,
        params: params.clone(),
    };
    // off log
    if params.scan_mode != ScanMode::Debug {
        log::set_max_level(log::LevelFilter::Off);
        if file_exists(G_DEFAULT_LOGFILE) {
            let _ = fs::remove_file(G_DEFAULT_LOGFILE);
        }
    }
    params.print_state.then(|| {
        print_start_info(&options)
    });

    let scan = Scanner::new(&options);
    scan.start().await;
    Ok(())
}
