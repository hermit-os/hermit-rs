//! Regression test for hermit-os/kernel#2433:
//! vsock listener cannot accept a second connection.
//!
//! The VM runs a server that accepts CONNECTIONS sequential connections,
//! reads "ping", writes "pong", and closes each one. The host-side client
//! is driven by xtask (see xtask/src/ci/qemu.rs test_vsock_server).
use std::io::{Read, Write};

#[cfg(target_os = "hermit")]
use hermit as _;

mod vsock;

use vsock::VsockListener;

const PORT: u32 = 9975;
pub const CONNECTIONS: usize = 2;

fn main() {
    println!("vsock_server_test: waiting for {CONNECTIONS} sequential connections on port {PORT}");

    let listener = VsockListener::bind(PORT).expect("bind failed");
    println!("[server] listening on port {PORT}");

    for i in 1..=CONNECTIONS {
        println!("[server] waiting for connection {i}/{CONNECTIONS}");
        let (mut stream, _addr) = listener.accept().expect("accept failed");
        println!("[server] accepted connection {i}");

        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).expect("read failed");
        let msg = std::str::from_utf8(&buf[..n]).unwrap_or("<invalid>");
        println!("[server] received: {msg:?}");
        assert_eq!(msg, "ping", "connection {i}: unexpected message");

        stream.write_all(b"pong").expect("write failed");
        println!("[server] sent pong for connection {i}");
        // stream drops here, closing the connection
    }

    println!("vsock_server_test: PASSED");
}
