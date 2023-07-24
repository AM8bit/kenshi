use std::fs::OpenOptions;
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{Ordering};
use std::{thread};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::time::Duration;
use crate::{G_LOOP_BREAK, G_RESPONSE};
use crate::common::{is_match};
use crate::data_type::{Matches, ScanMode, Stats};
use crate::error::stats_inc;
use console::Style;

pub struct ListenData {
    handles: Vec<JoinHandle<()>>,
    print_sender: Sender<String>,
    mode: ScanMode,
}

impl ListenData {
    pub fn new(print_sender: Sender<String>, mode: ScanMode)-> Self {
        Self {
            handles: vec![],
            print_sender,
            mode
        }
    }

    pub fn listen_data(&mut self, result_path: &str, custom_matches: Matches) {
        //jobs
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(result_path)
            .unwrap();
        let write_result = Arc::new(Mutex::new(file));
        for _ in 0..4 {
            let response_cloned = G_RESPONSE.clone();
            let loop_break_cloned = G_LOOP_BREAK.clone();
            let custom_matches = custom_matches.clone();
            let write_result = write_result.clone();
            let result_path = result_path.to_owned();
            let print_sender = self.print_sender.clone();
            let scan_mode = self.mode.clone();
            let handle = thread::spawn(move || {
                loop {
                    let resp;
                    {
                        let mut read_guard = response_cloned.write().unwrap();

                        if read_guard.queue.is_empty()
                            && loop_break_cloned.load(Ordering::SeqCst) == false
                        {
                            break;
                        }
                        resp = read_guard.dequeue();
                    }
                    if resp.is_none() {
                        thread::sleep(Duration::from_secs(1));
                        continue;
                    }

                    let resp = resp.unwrap();
                    let url = resp.url.to_string();
                    let mut print_line = String::new();
                    if scan_mode == ScanMode::DEBUG {
                        print_line = format!("[Status: {}, Size: {}, Lines: {}, Duration: {}ms]\n\t* {}",
                                resp.status, resp.html.len(), resp.html.lines().count(), resp.duration.as_millis(), url);
                    }
                    let match_ok = is_match(&resp, &custom_matches);
                    if match_ok.is_none() {
                        if !print_line.is_empty() {
                            let red = Style::new().red().bold();
                            print_sender.send(red.apply_to(print_line).to_string()).unwrap();
                        }
                        continue;
                    }
                    if scan_mode == ScanMode::DEBUG {
                        let green = Style::new().green().bold();
                        print_sender.send(green.apply_to(print_line).to_string()).unwrap();
                    }else {
                        print_sender.send(url.to_owned()).unwrap();
                    }

                    stats_inc(&Stats::Hits);
                    if !result_path.is_empty() {
                        let log_text = url.to_string() + "\n";
                        let mut file = write_result.lock().unwrap();
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

