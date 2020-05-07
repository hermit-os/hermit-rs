extern crate core_affinity;

use std::net::TcpStream;
use std::io::{Read, Write};
use std::io::ErrorKind::WouldBlock;
use config::Config;
use std::io;
use std::net::ToSocketAddrs;
use std::net::Shutdown;
use std::net::TcpListener;

/// Sends first n_bytes from wbuf using the given stream.
/// Make sure wbuf.len >= n_bytes
pub fn send_message(n_bytes: usize, stream: &mut TcpStream, wbuf: &Vec<u8>) {
    let mut send = 0;
    while send < n_bytes {
        match stream.write(&wbuf[send..]) {
            Ok(n) => send += n,
            Err(err) => match err.kind() {
                WouldBlock => {}
                _ => panic!("Error occurred while writing: {:?}", err),
            }
        }
    }
}

/// Reads n_bytes into rbuf from the given stream.
/// Make sure rbuf.len >= n_bytes
pub fn receive_message(n_bytes: usize, stream: &mut TcpStream, rbuf: &mut Vec<u8>) {
    // Make sure we receive the full buf back
    let mut recv = 0;
    while recv < n_bytes {
        match stream.read(&mut rbuf[recv..]) {
            Ok(n) => recv += n,
            Err(err) => match err.kind() {
                WouldBlock => {}
                _ => panic!("Error occurred while reading: {:?}", err),
            }
        }
    }
}

/// Setup the streams and eventually pins the thread according to the configuration.
pub fn setup(config: &Config, stream: &mut TcpStream) {
    if config.no_delay {
        stream.set_nodelay(true).expect("Can't set no_delay to true");
    }
    if config.non_blocking {
        stream.set_nonblocking(true).expect("Can't set channel to be non-blocking");
    }
}

pub fn client_connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
    return TcpStream::connect(addr);
}

pub fn close_connection(stream: &TcpStream) {
    stream.shutdown(Shutdown::Both).expect("shutdown call failed");
}

/// Starts listening on given port and return first connection to that port as a stream.
pub fn server_listen_and_get_first_connection(port: &String) -> TcpStream {
    let listener = TcpListener::bind("0.0.0.0:".to_owned() + port).unwrap();
    println!("Server running, listening for connection on 0.0.0.0:{}", port);
    let stream = listener.incoming().next().unwrap().unwrap();
    println!("Connection established with {:?}!", stream.peer_addr().unwrap());
    return stream;
}
