use std::collections::HashMap;
use std::io;
use std::io::Read;
use std::net::TcpListener;

use polling::{Event, Events, Poller};

fn main() -> io::Result<()> {
	let mut counter = 3;
	let mut streams = HashMap::new();
	let l1 = TcpListener::bind("0.0.0.0:9975")?;
	let l2 = TcpListener::bind("0.0.0.0:9976")?;
	l1.set_nonblocking(true)?;
	l2.set_nonblocking(true)?;

	let poller = Poller::new()?;
	unsafe {
		poller.add(&l1, Event::readable(1))?;
		poller.add(&l2, Event::readable(2))?;
	}

	println!("You can connect to the server using `nc`:");
	println!(" $ nc 127.0.0.1 9975");
	println!(" $ nc 127.0.0.1 9976");

	let mut events = Events::new();
	'outer: loop {
		events.clear();
		poller.wait(&mut events, None)?;

		for ev in events.iter() {
			match ev.key {
				1 => {
					println!("Accept on l1");
					let (stream, _) = l1.accept()?;
					unsafe {
						poller.add(&stream, Event::readable(counter))?;
					}
					streams.insert(counter, stream);
					counter = counter + 1;
					poller.modify(&l1, Event::readable(1))?;
				}
				2 => {
					println!("Accept on l2");
					let (stream, _) = l2.accept()?;
					unsafe {
						poller.add(&stream, Event::readable(counter))?;
					}
					streams.insert(counter, stream);
					counter = counter + 1;
					poller.modify(&l2, Event::readable(2))?;
				}
				_ => {
					if let Some(mut stream) = streams.get(&ev.key) {
						let mut buf = [0u8; 1000];
						let received = stream.read(&mut buf)?;
						poller.modify(&stream, Event::readable(ev.key))?;
						let msg = std::str::from_utf8(&buf[..received]).unwrap().trim_end();
						println!("{}", msg);
						if msg == "exit" {
							break 'outer;
						}
					} else {
						println!("Unknown key {}", ev.key);
					}
				}
			}
		}
	}

	for (_key, stream) in streams.drain().take(1) {
		poller.delete(&stream)?;
	}
	poller.delete(&l1)?;
	poller.delete(&l2)?;

	Ok(())
}
