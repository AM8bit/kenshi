use std::error::Error;

use hyper::Error as hyper_error;
use trust_dns_resolver::error::ResolveError;

use crate::data_type::Stats;
use crate::G_STATS;

pub fn stats_inc(stat: &Stats) {
    let mut value = G_STATS.get_mut(stat).unwrap();
    *value += 1;
}

pub fn stats_err_inc(e: &reqwest::Error) {
    if e.is_timeout() {
        stats_inc(&Stats::TimeOut);
        return;
    }

    if let Some(hyper_err) = e.source() {
        if let Some(connect_err) = &hyper_err.downcast_ref::<hyper_error>() {
            if let Some(e) = connect_err.source() {
                if let Some(e) = e.source() {
                    let resolve_err = e.downcast_ref::<ResolveError>();
                    if let Some(resolve_err) = resolve_err {
                        if let trust_dns_resolver::error::ResolveErrorKind::NoRecordsFound { .. } = resolve_err.kind() {
                            stats_inc(&Stats::DNSErr);
                            return;
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
        502 => &Stats::C502,
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
