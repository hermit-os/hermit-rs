use std::collections::BTreeMap;
use std::convert::TryInto;
use std::mem;
use std::ptr::{read_volatile, write_volatile};

use smoltcp::iface::{EthernetInterfaceBuilder, NeighborCache, Routes};
use smoltcp::phy::{self, Device, DeviceCapabilities};
use smoltcp::socket::SocketSet;
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};

use crate::net::NetworkInterface;
use x86::io::outl;

const SHAREDQUEUE_START: usize = 0x80000;
const UHYVE_NET_MTU: usize = 1500;
const UHYVE_QUEUE_SIZE: usize = 8;
const UHYVE_PORT_NETWRITE: u16 = 0x640;

extern "Rust" {
	fn sys_uhyve_get_ip() -> [u8; 4];
	fn sys_uhyve_get_gateway() -> [u8; 4];
	fn sys_uhyve_get_mask() -> [u8; 4];
	fn sys_uhyve_get_mac_address() -> [u8; 6];
}

pub fn is_network_available() -> bool {
	let myip = unsafe { sys_uhyve_get_ip() };
	if myip[0] == 0xff && myip[1] == 0xff && myip[2] == 0xff && myip[3] == 0xff {
		false
	} else {
		true
	}
}

impl NetworkInterface<UhyveNet> {
	pub fn new() -> Option<Self> {
		let myip = unsafe { sys_uhyve_get_ip() };
		if myip[0] == 0xff && myip[1] == 0xff && myip[2] == 0xff && myip[3] == 0xff {
			return None;
		}

		debug!("Initialize uhyve network interface!");

		let mygw = unsafe { sys_uhyve_get_gateway() };
		let mymask = unsafe { sys_uhyve_get_mask() };
		let mac = unsafe { sys_uhyve_get_mac_address() };

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
		let mut routes = Routes::new(BTreeMap::new());
		routes.add_default_ipv4_route(default_v4_gw).unwrap();

		/*info!("MAC address {}", ethernet_addr);
		info!("Configure network interface with address {}", ip_addrs[0]);
		info!("Configure gatway with address {}", default_v4_gw);*/

		let iface = EthernetInterfaceBuilder::new(device)
			.ethernet_addr(ethernet_addr)
			.neighbor_cache(neighbor_cache)
			.ip_addrs(ip_addrs)
			.routes(routes)
			.finalize();

		Some(Self {
			iface: iface,
			sockets: SocketSet::new(vec![]),
			channels: BTreeMap::new(),
			counter: 0,
			timestamp: Instant::now(),
		})
	}
}

#[repr(C)]
pub struct QueueInner {
	pub len: u16,
	pub data: [u8; UHYVE_NET_MTU + 34],
}

#[repr(C)]
pub struct SharedQueue {
	pub read: usize,
	pad0: [u8; 64 - 8],
	pub written: usize,
	pad1: [u8; 64 - 8],
	pub inner: [QueueInner; UHYVE_QUEUE_SIZE],
}

/// Data type to determine the mac address
#[derive(Debug, Default)]
#[repr(C)]
pub struct UhyveNet;

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

		let written = unsafe { read_volatile(&rx_queue.written) };
		let read = unsafe { read_volatile(&rx_queue.read) };
		let distance = written - read;

		if distance > 0 {
			let idx = read % UHYVE_QUEUE_SIZE;
			let len = unsafe { read_volatile(&rx_queue.inner[idx].len) };
			let tx = TxToken::new();
			let mut rx = RxToken::new(len);

			rx.buffer[0..len as usize].copy_from_slice(&rx_queue.inner[idx].data[0..len as usize]);

			unsafe { write_volatile(&mut rx_queue.read, read + 1) };

			Some((rx, tx))
		} else {
			None
		}
	}

	fn transmit(&'a mut self) -> Option<Self::TxToken> {
		trace!("create TxToken to transfer data");
		Some(TxToken::new())
	}
}

#[doc(hidden)]
pub struct RxToken {
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
pub struct TxToken;

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

		let written = unsafe { read_volatile(&tx_queue.written) };
		let read = unsafe { read_volatile(&tx_queue.read) };
		let distance = written - read;

		if distance < UHYVE_QUEUE_SIZE {
			let idx = written % UHYVE_QUEUE_SIZE;
			let result = f(&mut tx_queue.inner[idx].data[0..len]);

			if result.is_ok() == true {
				unsafe {
					write_volatile(&mut tx_queue.inner[idx].len, len.try_into().unwrap());
					write_volatile(&mut tx_queue.written, written + 1);
					outl(UHYVE_PORT_NETWRITE, 0);
				}
			} else {
				info!("Unable to consume packet");
			}

			result
		} else {
			info!("Drop packet!");
			Err(smoltcp::Error::Dropped)
		}
	}
}
