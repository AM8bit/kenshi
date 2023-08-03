use std::arch::x86_64::_rdtsc;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::thread::sleep;
use std::time::Duration;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{Rng, thread_rng};

//from scanner.rs
unsafe fn rnd_n() {
    if cfg!(target_arch = "x86_64") {
        _rdtsc() % 1000
    }else{
        thread_rng().gen::<u64>() % 1000
    };
}

//from scanner.rs
fn no_rnd_n() {
    sleep(Duration::from_millis(1))
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("rnd_n", |b| b.iter(|| unsafe { rnd_n() } ));
    c.bench_function("no_rnd_n", |b| b.iter(no_rnd_n ));
}


criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);