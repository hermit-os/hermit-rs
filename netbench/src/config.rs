use clap::{Arg, App};

pub struct Config {
    pub address: String,
    pub port: String,
    pub n_bytes: usize,
    pub n_rounds: usize,
    pub no_delay: bool,
    pub non_blocking: bool,
    pub p_id: i8,
}

impl Config {
    pub fn address_and_port(&self) -> String {
        format!("{}:{}", &self.address, &self.port)
    }
}
/// Extract the configuration from Command line
pub fn parse_config() -> Config {
    let matches = App::new("Config")
        .arg(Arg::with_name("address")
            .short("a")
            .long("address")
            .value_name("address")
            .help("IP4 address to connect to")
            .takes_value(true)
            .default_value("127.0.0.1")
        )
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .value_name("port")
            .help("port to connect to, like port 7878 if you wanna connect to 127.0.0.1:7878")
            .takes_value(true)
            .default_value("7878")
        )
        .arg(Arg::with_name("n_bytes")
            .short("k")
            .long("bytes")
            .value_name("n_bytes")
            .help("number of bytes to transfer every round")
            .takes_value(true)
            .default_value("1")
        )
        .arg(Arg::with_name("rounds")
            .short("r")
            .long("rounds")
            .value_name("rounds")
            .help("number of rounds of transfer to perform")
            .takes_value(true)
            .default_value("1000000")
        )
        .arg(Arg::with_name("nodelay")
            .short("d")
            .long("nodelay")
            .value_name("nodelay")
            .help("sets TCP in no-delay mode. Any int > 0 for true, 0 for false")
            .takes_value(true)
            .default_value("1")
        )
        .arg(Arg::with_name("nonblocking")
            .short("b")
            .long("nonblocking")
            .value_name("nonblocking")
            .help("sets TCP in non-blocking mode. Any int > 0 for true, 0 for false")
            .takes_value(true)
            .default_value("1")
        )
        .arg(Arg::with_name("thread")
            .short("t")
            .long("thread")
            .value_name("thread")
            .help("id of process to pin thread to, -1 for no pinning")
            .takes_value(true)
            .default_value("-1")
        )
        .get_matches();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    let address = String::from(matches.value_of("address").unwrap());
    let port = String::from(matches.value_of("port").unwrap());
    let n_bytes = matches.value_of("n_bytes").unwrap().parse::<usize>().unwrap();
    let n_rounds = matches.value_of("rounds").unwrap().parse::<usize>().unwrap();
    let no_delay = matches.value_of("nodelay").unwrap().parse::<usize>().unwrap() > 0;
    let non_blocking = matches.value_of("nonblocking").unwrap().parse::<usize>().unwrap() > 0;
    let p_id = matches.value_of("thread").unwrap().parse::<i8>().unwrap();

    // Don't kill machines
    if n_bytes > 100_000_000 {
        panic!("More than 100 MB per round is probably too much data you wanna send, \
        you may kill one of the machines. Try with maybe 100MB but more rounds")
    }

    // Very improbable case error handling
    if (n_bytes * 1000000) as u128 * n_rounds as u128 > u64::max_value().into() {
        panic!("There's gonna be too much data. Make sure n_bytes * n_rounds is < u128::MAX")
    }

    Config {
        address,
        port,
        n_bytes,
        n_rounds,
        no_delay,
        non_blocking,
        p_id
    }
}
