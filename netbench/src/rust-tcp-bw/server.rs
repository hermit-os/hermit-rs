#![allow(unused_imports)]

#[cfg(target_os = "hermit")]
extern crate hermit_sys;
extern crate bytes;
extern crate rust_tcp_io_perf;
extern crate hdrhist;

use rust_tcp_io_perf::connection;
use rust_tcp_io_perf::config;
use rust_tcp_io_perf::print_utils;
use std::io::Read;
use std::time::Instant;

fn main() {
    let args = config::parse_config();
    let n_bytes = args.n_bytes;
    let tot_n_bytes = (n_bytes * args.n_rounds) as u64;

    let mut buf = vec![0; n_bytes];
    let mut active = true;
    let mut hist = hdrhist::HDRHist::new();
    let mut tot_bytes: u64 = 0;
    let mut tot_bytes_stable: u64 = 0;
    let mut tot_time_stable: u64 = 0;

    let mut stream = connection::server_listen_and_get_first_connection(&args.port);

    let mut start = Instant::now();
    while active {
        let recv = stream.read(&mut buf).unwrap();
        if recv > 0 {
            let end = Instant::now();
            let duration = end.duration_since(start);
            let duration_ns = duration.as_secs() * 1_000_000_000u64 + duration.subsec_nanos() as u64;

            // Capture measures
            hist.add_value(duration_ns);
            tot_bytes += recv as u64;
            if tot_bytes > tot_n_bytes / 3 && tot_bytes < (tot_n_bytes * 2) / 3 {
                tot_bytes_stable += recv as u64;
                tot_time_stable += duration_ns;
            }

            start = end;
        } else {
            active = false;
        }
    }
    print!("{},{},{}", tot_bytes, tot_bytes_stable, tot_time_stable);
    print_utils::print_summary(hist);
    println!("Available approximated bandwidth: {} MB/s", tot_bytes_stable * 1000 / tot_time_stable);
}
