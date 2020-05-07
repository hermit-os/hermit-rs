extern crate hdrhist;

/// Prints dashed line
fn print_line() {
    println!("\n-------------------------------------------------------------\n");
}

/// Nicely outputs summary of execution with stats and CDF points.
pub fn print_summary(hist: hdrhist::HDRHist) {
    println!("Sent/received everything!");
    print_line();
    println!("HDRHIST summary, measure in ns");
    print_line();
    println!("summary:\n{:#?}", hist.summary().collect::<Vec<_>>());
    print_line();
    println!("Summary_string:\n{}", hist.summary_string());
    print_line();
    println!("CDF summary:\n");
    for entry in hist.ccdf_upper_bound() {
        println!("{:?}", entry);
    }
}
