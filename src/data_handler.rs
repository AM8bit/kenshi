use std::fs::OpenOptions;
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use console::Style;
use script::ScriptEngine;

use crate::{G_LOOP_BREAK, G_RESPONSE};
use crate::common::{is_filter, is_match};
use crate::data_type::{FilterRules, Matches, ScanMode, ScriptOpt, Stats};
use crate::error::stats_inc;

#[derive(Default)]
struct PrintData {
    status: u16,
    ipv4: String,
    size: usize,
    lines: usize,
    duration: u128,
    url: String,
    script_output: String,
}

struct MsgSender {
    sender: Sender<String>,
    msg: String,
    print_opt: Option<PrintData>,
}

impl MsgSender {
    pub fn new(sender: Sender<String>)-> Self {
        Self {
            sender,
            msg: "".to_string(),
            print_opt: None,
        }
    }
    pub fn set_msg(&mut self, msg: String)-> &Self {
        self.msg = msg;
        self
    }

    pub fn set_script_output(&mut self, msg: String)-> &Self {
        if let Some( opt) = &mut self.print_opt {
            opt.script_output = msg;
        }
        self
    }

    pub fn debug(&mut self, data: PrintData) -> &Self {
        self.print_opt = Some(data);
        self
    }

    pub fn send_color(&self, color: Style)-> &Self {
        if let Some(debug) = &self.print_opt {
            let mut output = format!("[Status: {}, Size: {}, Lines: {}, Duration: {}ms, IP: {}]\n\t* {}",
                                           debug.status, debug.size, debug.lines, debug.duration, debug.ipv4, debug.url);
            if !debug.script_output.is_empty() {
                output += &format!("\toutput: {}", debug.script_output)
            }
            //panic!("{}", output);
            self.sender.send(color.apply_to(output).to_string()).unwrap();
            return self
        }
        if !self.msg.is_empty() {
            self.sender.send(color.apply_to(&self.msg).to_string()).unwrap();
        }
        self
    }

    pub fn send(&self) {
        if !self.msg.is_empty() {
            self.sender.send(self.msg.clone()).unwrap();
        }
    }
}

pub struct ListenData {
    handles: Vec<JoinHandle<()>>,
    print_sender: Sender<String>,
    mode: ScanMode,
    script: Option<ScriptOpt>,
    result_outfile: Option<String>
}

impl ListenData {
    pub fn new(print_sender: Sender<String>, mode: ScanMode) -> Self {
        Self {
            handles: vec![],
            print_sender,
            mode,
            script: None,
            result_outfile: None,
        }
    }

    pub fn use_script(&mut self, script: ScriptOpt) {
        self.script = Some(script);
    }

    pub fn save_as(&mut self, outfile: &str) {
        self.result_outfile = Some(outfile.to_string())
    }

    pub fn handler(&mut self, custom_matches: Option<Matches>, custom_filters: Option<FilterRules>) {
        //jobs
        let file = match &self.result_outfile {
            Some(p) => {
                let  file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(p)
                    .unwrap();
                Some(file)
            }
            _ => {
                None
            }
        };

        let outfile = Arc::new(Mutex::new(file));
        for _ in 0..4 {
            let response_cloned = G_RESPONSE.clone();
            let loop_break_cloned = G_LOOP_BREAK.clone();
            let custom_matches = custom_matches.clone();
            let custom_filters = custom_filters.clone();
            let outfile = outfile.clone();
            let print_sender = self.print_sender.clone();
            let scan_mode = self.mode.clone();
            let script_opt = self.script.clone();
            let handle = thread::spawn(move || {
                let mut script_ctx: Option<ScriptEngine> = None;
                if let Some(opt) = script_opt {
                    let script_engine = ScriptEngine::new(&opt.script_path);
                    match script_engine {
                        Ok(script) => {
                            script_ctx = Some(script);
                        }
                        Err(e) => log::error!("{e}")
                    }
                }
                let mut msg_sender = MsgSender::new(print_sender);
                loop {
                    let resp;
                    {
                        let mut read_guard = response_cloned.write().unwrap();

                        if read_guard.queue.is_empty()
                            && !loop_break_cloned.load(Ordering::SeqCst)
                        {
                            break;
                        }
                        resp = read_guard.dequeue();
                    }
                    if resp.is_none() {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }

                    let resp = resp.unwrap();
                    let url = resp.url.to_string();
                    let html = String::from_utf8_lossy(resp.html.as_slice());

                    if scan_mode == ScanMode::Debug {
                        let mut print_data = PrintData::default();
                        let mut ip = String::from("None");
                        if let Some(s) = resp.remote_addr {
                            ip = s.ip().to_string();
                        }
                        print_data.ipv4 = ip;
                        print_data.status = resp.status;
                        print_data.size = resp.html.len();
                        print_data.lines = resp.html.lines().count();
                        print_data.duration = resp.duration.as_millis();
                        print_data.url = url.clone();
                        msg_sender.debug(print_data);
                    }
                    // filter response body
                    if let Some(filters) = &custom_filters {
                        if is_filter(&html, filters) {
                            if scan_mode == ScanMode::Debug {
                                //msg_sender.send_color(Style::new().red().bold());
                            }
                            continue
                        }
                    }
                    // match response body
                    if let Some(matches) = &custom_matches {
                        if !is_match(&html, matches) {
                            if scan_mode == ScanMode::Debug {
                                msg_sender.send_color(Style::new().red().bold());
                            }
                            continue;
                        }
                    }

                    // if using script
                    if let Some(engine) = &script_ctx {
                        match engine.run_script(html.parse().unwrap()) {
                            Ok(script_output) => {
                                msg_sender.set_script_output(script_output);
                            }
                            Err(e) => log::error!("{}", e.to_string())
                        }
                    }

                    if scan_mode == ScanMode::Debug {
                        let color = match &resp.status {
                            200 => Style::new().green().bold(),
                            301 => Style::new().blue(),
                            404 => Style::new().dim().bold(),
                            403 => Style::new().yellow().bold(),
                            500 => Style::new().red().bold(),
                            400 => Style::new().dim().bold(),
                            401 => Style::new().blue().bold(),
                            _ => Style::new().cyan().bold(),
                        };
                        msg_sender.send_color(color);
                    } else {
                        msg_sender.set_msg(url.to_string()).send();
                    }
                    drop(resp);
                    stats_inc(&Stats::Hits);
                    let mut file = outfile.lock().unwrap();
                    if let Some(ref mut file) = *file {
                            let log_text = url.to_string() + "\n";
                            let write_str = log_text.to_string();
                            file.write_all(write_str.as_bytes()).unwrap();
                            file.flush().unwrap();
                        }
                    }
            });
            self.handles.push(handle)
        }
    }

    pub fn waiting(self) {
        // waiting for stored.
        for handle in self.handles {
            handle.join().unwrap();
        }
    }
}

