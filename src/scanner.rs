


use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use futures::{stream, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
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
        if !self.options.print_state {
            status_bar.finish_and_clear();
            stats_bar.finish_and_clear();
        }
        (pb, status_bar, stats_bar)
    }

    pub fn refresh_pb(&self, status_bar: &ProgressBar, stats_bar: &ProgressBar) {
        status_bar.set_message(format!("200: {}, 404: {}, 301: {}, 302: {}, 403: {}, 401: {}, 500: {}",
                                       &G_STATS.get(&Stats::C200).unwrap().to_owned(),
                                       &G_STATS.get(&Stats::C404).unwrap().to_owned(),
                                       &G_STATS.get(&Stats::C301).unwrap().to_owned(),
                                       &G_STATS.get(&Stats::C302).unwrap().to_owned(),
                                       &G_STATS.get(&Stats::C403).unwrap().to_owned(),
                                       &G_STATS.get(&Stats::C401).unwrap().to_owned(),
                                       &G_STATS.get(&Stats::C500).unwrap().to_owned(),
        ));
        let jobs_len = G_RESPONSE.read().unwrap().queue.len();
        stats_bar.set_message(format!("Hits: {}, , Jobs: {}, TO: {}, IO: {}, DNS: {}, Mem: {}",
                                      &G_STATS.get(&Stats::Hits).unwrap().to_owned(),
                                      jobs_len,
                                      &G_STATS.get(&Stats::TimeOut).unwrap().to_owned(),
                                      &G_STATS.get(&Stats::IOErr).unwrap().to_owned(),
                                      &G_STATS.get(&Stats::DNSErr).unwrap().to_owned(),
                                      "60%"
        ));
        stats_bar.inc(1);
        status_bar.inc(1);
    }

    pub fn client_build(&self)-> Option<Client> {
        let mut headers = HeaderMap::new();
        let options = self.options;
        headers.insert(header::USER_AGENT, options.user_agent.parse().unwrap());
        headers.insert(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9".parse().unwrap());

        let (config, opts) = read_system_conf().unwrap();
        let _dns = trust_dns_resolver::TokioAsyncResolver::tokio(config, opts).unwrap();
        //let test: Arc<dyn Resolve> = Arc::new(MyResolver);
        let mut client = Client::builder()
            .use_rustls_tls()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(options.request_timeout))
            .connect_timeout(Duration::from_secs(options.request_timeout))
            .default_headers(headers)
            .redirect(redirect::Policy::none())
            .http1_only()
            .trust_dns(true)
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .tcp_nodelay(true)
            .tcp_keepalive(None);
            //.dns_resolver(Arc::new(TrustDnsResolver::new().map_err(crate::error::builder)?));

        if !options.proxy_server.is_empty() {
            match reqwest::Proxy::all(options.proxy_server.clone()) {
                Ok(mut p) => {
                    p = p.basic_auth(&options.proxy_user, &options.proxy_pass);
                    client = client.proxy(p);
                },
                Err(e) => {
                    println!("{}", e.to_string());
                    return None
                }
            }
        }
        Some(client.build().unwrap())
    }

    pub async fn start(self) {
        let options = self.options.clone();
        use std::sync::mpsc::channel;
        let (pr_tx, pr_rx): (Sender<String>, Receiver<String>) = channel();

        let mut listen_data = ListenData::new(pr_tx, options.scan_mode);
        listen_data.listen_data(&options.result_file, options.custom_matches.clone());
        let client = self.client_build();
        let client = match client {
            Some(c) => c,
            None => panic!("http client failed to initialize.")
        };
        let bodies = stream::iter(options.wordlist.clone()).map(|payload| {
            let client = client.clone();
            let fuzz_url = options.fuzz_url.replace("FUZZ", &payload);
            //let stats = stats.clone();
            tokio::spawn(async move {
                for _ in 0..options.request_retries {
                    let start = Instant::now();
                    let resp = client.get(&fuzz_url).send().await;
                    match resp {
                        Ok(r) => {
                            let status = r.status().as_u16();
                            stats_code_inc(&status);
                            let real_url = r.url().to_string();
                            if let Ok(data) = &r.bytes().await {
                                let duration = start.elapsed();
                                return Some(HttpResp {
                                    status,
                                    url: real_url,
                                    html: data.to_vec(),
                                    duration
                                });
                            }
                        }
                        Err(e) => {
                            stats_err_inc(&e);
                            log::error!("{}", e.to_string());
                        }
                    }
                }
                None
            })
        })
            .buffer_unordered(options.concurrent_num);

        let deps = options.wordlist.len() as u64;
        let (pb, status_bar, stats_bar) = self.install_pb(deps);
        //let status_bar = status_bar.unwrap();
        //let stats_bar = stats_bar.unwrap();
        bodies.for_each(|resp| async {
            pb.inc(1);
            if let Ok(msg) = pr_rx.try_recv() {
                pb.println(msg);
            }
            if options.print_state {
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
        pb.finish_and_clear();
    }
}