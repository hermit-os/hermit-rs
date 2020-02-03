use std::collections::BTreeMap;
use std::convert::TryInto;
use std::mem;
use std::ptr::{read_volatile, write_volatile};
use std::sync::atomic::{AtomicUsize, Ordering};

use smoltcp::iface::{EthernetInterfaceBuilder, NeighborCache, Routes};
use smoltcp::phy::{self, Device, DeviceCapabilities};
use smoltcp::socket::SocketSet;
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};

use x86::io::outl;

const SHAREDQUEUE_START: usize = 0x80000;
const UHYVE_NET_MTU: usize = 1500;
const UHYVE_QUEUE_SIZE: usize = 8;
const UHYVE_PORT_NETWRITE: u16 = 0x640;

pub type Tid = u32;

extern "Rust" {
	fn uhyve_get_ip() -> [u8; 4];
	fn uhyve_get_gateway() -> [u8; 4];
	fn uhyve_get_mask() -> [u8; 4];
	fn uhyve_get_mac_address() -> [u8; 6];
	fn uhyve_is_polling() -> bool;
	fn uhyve_netwait(millis: Option<u64>);
	fn uhyve_set_polling(mode: bool);
}

extern "C" {
	fn sys_spawn(
		id: *mut Tid,
		func: extern "C" fn(usize),
		arg: usize,
		prio: u8,
		selector: isize,
	) -> i32;
}

#[no_mangle]
extern "C" fn uhyve_thread(_: usize) {
	debug!("Initialize uhyve network interface!");

	let myip = unsafe { uhyve_get_ip() };
	if myip[0] == 0xff && myip[1] == 0xff && myip[2] == 0xff && myip[3] == 0xff {
		panic!("Unable to determine IP address");
	}

	let mygw = unsafe { uhyve_get_gateway() };
	let mymask = unsafe { uhyve_get_mask() };
	let mac = unsafe { uhyve_get_mac_address() };

	// calculate the netmask length
	// => count the number of contiguous 1 bits,
	// starting at the most significant bit in the first octet
	let mut prefix_len = (!mymask[0]).trailing_zeros();
	if prefix_len == 8 {
		prefix_len += (!mymask[1]).trailing_zeros();
	}
	if prefix_len == 16 {
		prefix_len += (!mymask[2]).trailing_zeros();
	}
	if prefix_len == 24 {
		prefix_len += (!mymask[3]).trailing_zeros();
	}

	let device = UhyveNet {};
	let neighbor_cache = NeighborCache::new(BTreeMap::new());
	let ethernet_addr = EthernetAddress([mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]]);
	let ip_addrs = [IpCidr::new(
		IpAddress::v4(myip[0], myip[1], myip[2], myip[3]),
		prefix_len.try_into().unwrap(),
	)];
	let default_v4_gw = Ipv4Address::new(mygw[0], mygw[1], mygw[2], mygw[3]);
	let mut routes_storage = [None; 1];
	let mut routes = Routes::new(&mut routes_storage[..]);
	routes.add_default_ipv4_route(default_v4_gw).unwrap();

	info!("MAC address {}", ethernet_addr);
	info!("Configure network interface with address {}", ip_addrs[0]);
	info!("Configure gatway with address {}", default_v4_gw);

	let mut iface = EthernetInterfaceBuilder::new(device)
		.ethernet_addr(ethernet_addr)
		.neighbor_cache(neighbor_cache)
		.ip_addrs(ip_addrs)
		.routes(routes)
		.finalize();

	let mut sockets = SocketSet::new(vec![]);
	let mut counter: usize = 0;
	loop {
		let timestamp = Instant::now();

		iface
			.poll(&mut sockets, timestamp)
			.map(|_| {
				trace!("receive message {}", counter);
				counter += 1;
			})
			.unwrap_or_else(|e| debug!("Poll: {:?}", e));

		if unsafe { !uhyve_is_polling() } {
			let delay = match iface.poll_delay(&sockets, timestamp) {
				Some(duration) => {
					if duration.millis() > 0 {
						Some(duration.millis())
					} else {
						Some(1)
					}
				}
				None => None,
			};

			unsafe {
				uhyve_netwait(delay);
			}
		}
	}
}

pub fn network_init() {
	let mut tid: Tid = 0;
	let ret = unsafe { sys_spawn(&mut tid, uhyve_thread, 0, 3, 0) };
	if ret >= 0 {
		debug!("Spawn network thread with id {}", tid);
	}
}

#[repr(C)]
pub struct QueueInner {
	pub len: u16,
	pub data: [u8; UHYVE_NET_MTU],
}

#[repr(C)]
pub struct SharedQueue {
	pub read: AtomicUsize,
	pub written: AtomicUsize,
	pub inner: [QueueInner; UHYVE_QUEUE_SIZE],
}

/// Data type to determine the mac address
#[derive(Debug, Default)]
#[repr(C)]
struct UhyveNet;

impl<'a> Device<'a> for UhyveNet {
	type RxToken = RxToken;
	type TxToken = TxToken;

	fn capabilities(&self) -> DeviceCapabilities {
		let mut cap = DeviceCapabilities::default();
		cap.max_transmission_unit = UHYVE_NET_MTU;
		cap
	}

	fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
		let rx_queue = unsafe { &mut *(SHAREDQUEUE_START as *mut u8 as *mut SharedQueue) };
		let written = rx_queue.written.load(Ordering::SeqCst);
		let read = rx_queue.read.load(Ordering::SeqCst);
		let distance = written - read;

		if distance > 0 {
			let idx = read % UHYVE_QUEUE_SIZE;
			let len = unsafe { read_volatile(&rx_queue.inner[idx].len) };
			let tx = TxToken::new();
			let mut rx = RxToken::new(len);

			rx.buffer[0..len as usize].copy_from_slice(&rx_queue.inner[idx].data[0..len as usize]);
			rx_queue.read.fetch_add(1, Ordering::SeqCst);

			Some((rx, tx))
		} else {
			trace!("Disable polling mode");

			unsafe {
				uhyve_set_polling(false);
			}

			None
		}
	}

	fn transmit(&'a mut self) -> Option<Self::TxToken> {
		trace!("create TxToken to transfer data");
		Some(TxToken::new())
	}
}

#[doc(hidden)]
struct RxToken {
	buffer: [u8; UHYVE_NET_MTU],
	len: u16,
}

impl RxToken {
	pub fn new(len: u16) -> RxToken {
		RxToken {
			buffer: [0; UHYVE_NET_MTU],
			len: len,
		}
	}
}

impl phy::RxToken for RxToken {
	fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> smoltcp::Result<R>
	where
		F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
	{
		let (first, _) = self.buffer.split_at_mut(self.len as usize);
		f(first)
	}
}

#[doc(hidden)]
struct TxToken;

impl TxToken {
	pub const fn new() -> Self {
		TxToken {}
	}
}

impl phy::TxToken for TxToken {
	fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
	where
		F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
	{
		let tx_queue = unsafe {
			&mut *((SHAREDQUEUE_START + mem::size_of::<SharedQueue>()) as *mut u8
				as *mut SharedQueue)
		};
		let written = tx_queue.written.load(Ordering::SeqCst);
		let read = tx_queue.read.load(Ordering::SeqCst);
		let distance = written - read;

		if distance < UHYVE_QUEUE_SIZE {
			let idx = written % UHYVE_QUEUE_SIZE;
			let result = f(&mut tx_queue.inner[idx].data[0..len]);

			if result.is_ok() == true {
				unsafe {
					write_volatile(&mut tx_queue.inner[idx].len, len.try_into().unwrap());
				}
				tx_queue.written.fetch_add(1, Ordering::SeqCst);

				if distance == 0 {
					unsafe {
						outl(UHYVE_PORT_NETWRITE, 0);
					}
				}
			}

			result
		} else {
			Err(smoltcp::Error::Dropped)
		}
	}
}
