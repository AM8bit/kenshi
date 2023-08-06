use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::sleep;
use std::time::Duration;

use futures::{stream, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::{Rng, thread_rng};
use reqwest::{Client, header, redirect};
use reqwest::header::HeaderMap;
use tokio::time::Instant;
use trust_dns_resolver::system_conf::read_system_conf;

use crate::{G_LOOP_BREAK, G_RESPONSE, G_STATS, HttpResp};
use crate::data_handler::ListenData;
use crate::data_type::*;
use crate::error::{stats_code_inc, stats_err_inc};

pub struct Scanner<'a> {
    options: &'a Options<'a>,
    mpg: MultiProgress,
}


impl<'a> Scanner<'a> {
    pub fn new(options: &'a Options) -> Self {
        Self {
            options,
            mpg: MultiProgress::new(),
        }
    }

    pub fn install_pb(&self, deps: u64) -> (ProgressBar, ProgressBar, ProgressBar) {
        let http_spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀");
        let stats_spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg} elapsed: {elapsed_precise}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀ ");
        let pb = self.mpg.add(ProgressBar::new(deps));
        let status_bar = self.mpg.insert_before(&pb, ProgressBar::new(deps));
        status_bar.set_prefix("HTTP");
        status_bar.set_style(http_spinner_style);
        let stats_bar = self.mpg.insert_before(&pb, ProgressBar::new(deps));
        stats_bar.set_prefix("Stats");
        stats_bar.set_style(stats_spinner_style);
        pb.set_position(0);
        status_bar.set_position(0);
        stats_bar.set_position(0);
        status_bar.set_message("waiting...".to_string());
        stats_bar.set_message("waiting...".to_string());
        if !self.options.params.print_state {
            status_bar.finish_and_clear();
            stats_bar.finish_and_clear();
        }
        (pb, status_bar, stats_bar)
    }

    pub fn refresh_pb(&self, status_bar: &ProgressBar, stats_bar: &ProgressBar) {
        if !self.options.params.print_state {
            return;
        }
        status_bar.set_message(format!("200: {}, 404: {}, 301: {}, 302: {}, 403: {}, 401: {}, 500: {}, 502: {}",
                                       &G_STATS.get(&Stats::C200).unwrap().to_string(),
                                       &G_STATS.get(&Stats::C404).unwrap().to_string(),
                                       &G_STATS.get(&Stats::C301).unwrap().to_string(),
                                       &G_STATS.get(&Stats::C302).unwrap().to_string(),
                                       &G_STATS.get(&Stats::C403).unwrap().to_string(),
                                       &G_STATS.get(&Stats::C401).unwrap().to_string(),
                                       &G_STATS.get(&Stats::C500).unwrap().to_string(),
                                       &G_STATS.get(&Stats::C502).unwrap().to_string(),
        ));
        let jobs_len = G_RESPONSE.read().unwrap().queue.len();
        stats_bar.set_message(format!("Hits: {}, , Jobs: {}, TO: {}, IO: {}, DNS: {}",
                                      &G_STATS.get(&Stats::Hits).unwrap().to_string(),
                                      jobs_len,
                                      &G_STATS.get(&Stats::TimeOut).unwrap().to_string(),
                                      &G_STATS.get(&Stats::IOErr).unwrap().to_string(),
                                      &G_STATS.get(&Stats::DNSErr).unwrap().to_string(),
        ));
        stats_bar.inc(1);
        status_bar.inc(1);
    }

    pub fn client_build(&self) -> Option<Client> {
        let mut headers = HeaderMap::new();
        let options = self.options;
        headers.insert(header::USER_AGENT, options.params.user_agent.parse().unwrap());
        headers.insert(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9".parse().unwrap());

        let (config, opts) = read_system_conf().unwrap();
        let _dns = trust_dns_resolver::TokioAsyncResolver::tokio(config, opts).unwrap();
        let mut client = Client::builder()
            .use_rustls_tls()
            .http1_title_case_headers() // Optimize HTTP/1 header formatting for better performance
            .http2_initial_stream_window_size(Some(65535)) // Increase the initial HTTP/2 stream window size for better throughput
            .http2_initial_connection_window_size(Some(1048576)) // Increase the initial HTTP/2 connection window size for better throughput
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(options.params.request_timeout))
            .connect_timeout(Duration::from_secs(options.params.request_timeout))
            .default_headers(headers)
            .http1_only()
            .trust_dns(true)
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .tcp_nodelay(true)
            .tcp_keepalive(None);
        //.dns_resolver(Arc::new(TrustDnsResolver::new().map_err(crate::error::builder)?));
        if options.params.follow_redirect == 0 {
            client = client.redirect(redirect::Policy::none())
        } else {
            client = client.redirect(redirect::Policy::limited(options.params.follow_redirect))
        }
        if !options.params.proxy_server.is_empty() {
            match reqwest::Proxy::all(options.params.proxy_server.clone()) {
                Ok(mut p) => {
                    p = p.basic_auth(&options.params.proxy_user, &options.params.proxy_pass);
                    client = client.proxy(p);
                }
                Err(e) => {
                    println!("{}", e.to_string());
                    return None;
                }
            }
        }
        Some(client.build().unwrap())
    }

    pub async fn start(self) {
        let options = self.options.clone();
        use std::sync::mpsc::channel;
        let (pr_tx, pr_rx): (Sender<String>, Receiver<String>) = channel();

        let mut listen_data = ListenData::new(pr_tx, options.params.scan_mode);
        if let Some(opt) = options.params.script_option {
            listen_data.use_script(opt);
        }
        if let Some(p) =&options.params.result_file {
            listen_data.save_as(p)
        }
        let matches = options.params.custom_matches.clone();
        let filters = options.params.custom_filters.clone();
        listen_data.handler( matches, filters);

        let client = self.client_build();
        let client = match client {
            Some(c) => c,
            None => panic!("http client failed to initialize.")
        };
        let bodies = stream::iter(options.params.wordlist).map(|payload| {
            let client = client.clone();
            let fuzz_url = options.params.fuzz_url.replace("FUZZ", &payload);
            let match_status = match options.params.custom_matches.clone() {
                Some(s) => s.status_code,
                None=> None,
            };
            let filter_status = match options.params.custom_filters.clone() {
                Some(s) => s.status_code,
                None=> None,
            };

            tokio::spawn(async move {
                for _ in 0..options.params.request_retries {
                    let start = Instant::now();
                    let resp = client.get(&fuzz_url).send().await;
                    match resp {
                        Ok(r) => {
                            let status = r.status().as_u16();
                            stats_code_inc(&status);
                            // Prioritize invalid states
                            // This will discard excluded prints
                            if let Some(s) = &filter_status {
                                if s.contains(&status) {
                                    return None;
                                }
                            }
                            if let Some(s) = &match_status {
                                if !s.contains(&status) {
                                    return None;
                                }
                            }
                            let real_url = r.url().to_string();
                            let remote_addr = r.remote_addr();
                            match &r.bytes().await {
                                Ok(data) => {
                                    let duration = start.elapsed();
                                    return Some(HttpResp {
                                        status,
                                        url: real_url,
                                        html: data.to_vec(),
                                        duration,
                                        remote_addr,//Real ip acquisition, needs some improvement
                                    });
                                }
                                Err(e) => {
                                    log::info!("{}", e.to_string());
                                }
                            }
                        }
                        Err(e) => {
                            stats_err_inc(&e);
                            log::error!("{} {}", fuzz_url, e.to_string());
                        }
                    }
                }
                None
            })
        }).buffer_unordered(options.params.concurrent_num);

        let deps = options.params.wordlist_len as u64;
        let (pb, status_bar, stats_bar) = self.install_pb(deps);
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::_rdtsc;
        bodies.for_each(|resp| async {
            pb.inc(1);
            if let Ok(msg) = pr_rx.try_recv() {
                pb.println(msg);
            }
            if self.options.params.wordlist_len > 10000 {
                // everybody's busy
                unsafe {
                    let unr = if cfg!(target_arch = "x86_64") {
                        _rdtsc() % 100
                    }else{
                        thread_rng().gen::<u64>() % 100
                    };
                    if unr == 0 {
                        self.refresh_pb(&status_bar, &stats_bar);
                    }
                }
            }else {
                self.refresh_pb(&status_bar, &stats_bar);
            }

            if let Ok(Some(resp)) = resp {
                let mut resp_write = G_RESPONSE.write().unwrap();
                resp_write.enqueue(resp);
            }
        }).await;
        //send over signal
        G_LOOP_BREAK.store(false, Ordering::SeqCst);
        listen_data.waiting();
        // Completion of final work
        while let Ok(msg) = pr_rx.try_recv() {
            pb.println(msg);
            self.refresh_pb(&status_bar, &stats_bar);
        }
        if self.options.params.scan_mode == ScanMode::Debug {
            sleep(Duration::from_secs(3));
        }
        pb.finish_and_clear();
    }
}