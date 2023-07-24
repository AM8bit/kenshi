use std::error::Error;



use crate::data_type::Stats;
use crate::G_STATS;
use hyper::Error as hyper_error;
use trust_dns_resolver::error::ResolveError;


pub fn stats_inc(stat: &Stats) {
    let mut value = G_STATS.get_mut(stat).unwrap();
    *value += 1;
}

pub fn stats_err_inc(e: &reqwest::Error) {
    if e.is_timeout() {
        stats_inc(&Stats::TimeOut);
        return
    }

    if let Some(hyper_err) = e.source() {
        if let Some(connect_err) = &hyper_err.downcast_ref::<hyper_error>() {
            if let Some(e) = connect_err.source() {
                if let Some(e) = e.source() {
                    let resolve_err = e.downcast_ref::<ResolveError>();
                    if let Some(resolve_err) = resolve_err {
                        if let trust_dns_resolver::error::ResolveErrorKind::NoRecordsFound { .. } = resolve_err.kind() {
                            stats_inc(&Stats::DNSErr);
                            return
                        }
                    }
                }
            }
        }
    }

    if e.is_request() {
        stats_inc(&Stats::IOErr);
    }
}

pub fn stats_code_inc(stat: &u16) {
    let code = match stat {
        500 => &Stats::C500,
        200 => &Stats::C200,
        404 => &Stats::C404,
        403 => &Stats::C403,
        401 => &Stats::C401,
        301 => &Stats::C301,
        302 => &Stats::C302,
        _ => &Stats::C000,
    };
    stats_inc(code)
}
/*
[src/error.rs:25] &s = Some(
    hyper::Error(
        Connect,
        ConnectError(
            "dns error",
            ResolveError {
                kind: NoRecordsFound {
                    query: Query {
                        name: Name("fmprc.gov.cn.localdomain."),
                        query_type: AAAA,
                        query_class: IN,
                    },
                    soa: Some(
                        Record {
                            name_labels: Name("."),
                            rr_type: SOA,
                            dns_class: IN,
                            ttl: 86357,
                            rdata: Some(
                                SOA(
                                    SOA {
                                        mname: Name("a.root-servers.net."),
                                        rname: Name("nstld.verisign-grs.com."),
                                        serial: 2023072100,
                                        refresh: 1800,
                                        retry: 900,
                                        expire: 604800,
                                        minimum: 86400,
                                    },
                                ),
                            ),
 */