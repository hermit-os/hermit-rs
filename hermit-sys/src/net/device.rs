include!(concat!(env!("OUT_DIR"), "/constants.rs"));

use std::collections::BTreeMap;
use std::convert::TryInto;
use std::net::Ipv4Addr;
use std::slice;

use smoltcp::iface::{EthernetInterfaceBuilder, NeighborCache, Routes};
#[cfg(feature = "trace")]
use smoltcp::phy::EthernetTracer;
use smoltcp::phy::{self, Device, DeviceCapabilities};
use smoltcp::socket::SocketSet;
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};

use crate::net::NetworkInterface;

extern "Rust" {
	fn sys_get_mac_address() -> Result<[u8; 6], ()>;
	fn sys_get_mtu() -> Result<u16, ()>;
	fn sys_get_tx_buffer(len: usize) -> Result<(*mut u8, usize), ()>;
	fn sys_send_tx_buffer(handle: usize, len: usize) -> Result<(), ()>;
	fn sys_receive_rx_buffer() -> Result<&'static mut [u8], ()>;
	fn sys_rx_buffer_consumed() -> Result<(), ()>;
	fn sys_free_tx_buffer(handle: usize);
}

/// Data type to determine the mac address
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct HermitNet {
	pub mtu: u16,
}

impl HermitNet {
	pub fn new(mtu: u16) -> Self {
		Self { mtu }
	}
}

impl NetworkInterface<HermitNet> {
	pub fn new() -> Option<Self> {
		let mtu = match unsafe { sys_get_mtu() } {
			Ok(mtu) => mtu,
			Err(_) => {
				return None;
			}
		};
		let device = HermitNet::new(mtu);
		#[cfg(feature = "trace")]
		let device = EthernetTracer::new(device, |_timestamp, printer| {
			trace!("{}", printer);
		});

		let mac: [u8; 6] = match unsafe { sys_get_mac_address() } {
			Ok(mac) => mac,
			Err(_) => {
				return None;
			}
		};
		let myip: Ipv4Addr = HERMIT_IP.parse().expect("Unable to parse IPv4 address");
		let myip = myip.octets();
		let mygw: Ipv4Addr = HERMIT_GATEWAY
			.parse()
			.expect("Unable to parse IPv4 address");
		let mygw = mygw.octets();
		let mymask: Ipv4Addr = HERMIT_MASK.parse().expect("Unable to parse IPv4 address");
		let mymask = mymask.octets();

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

		let neighbor_cache = NeighborCache::new(BTreeMap::new());
		let ethernet_addr = EthernetAddress([mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]]);
		let ip_addrs = [IpCidr::new(
			IpAddress::v4(myip[0], myip[1], myip[2], myip[3]),
			prefix_len.try_into().unwrap(),
		)];
		let default_v4_gw = Ipv4Address::new(mygw[0], mygw[1], mygw[2], mygw[3]);
		let mut routes = Routes::new(BTreeMap::new());
		routes.add_default_ipv4_route(default_v4_gw).unwrap();

		info!("MAC address {}", ethernet_addr);
		info!("Configure network interface with address {}", ip_addrs[0]);
		info!("Configure gateway with address {}", default_v4_gw);
		info!("MTU: {} bytes", mtu);

		let iface = EthernetInterfaceBuilder::new(device)
			.ethernet_addr(ethernet_addr)
			.neighbor_cache(neighbor_cache)
			.ip_addrs(ip_addrs)
			.routes(routes)
			.finalize();

		Some(Self {
			iface,
			sockets: SocketSet::new(vec![]),
			wait_for: BTreeMap::new(),
			timestamp: Instant::now(),
		})
	}
}

impl<'a> Device<'a> for HermitNet {
	type RxToken = RxToken;
	type TxToken = TxToken;

	fn capabilities(&self) -> DeviceCapabilities {
		let mut cap = DeviceCapabilities::default();
		cap.max_transmission_unit = self.mtu.into();
		cap
	}

	fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
		match unsafe { sys_receive_rx_buffer() } {
			Ok(buffer) => Some((RxToken::new(buffer), TxToken::new())),
			_ => None,
		}
	}

	fn transmit(&'a mut self) -> Option<Self::TxToken> {
		trace!("create TxToken to transfer data");
		Some(TxToken::new())
	}
}

#[doc(hidden)]
pub struct RxToken {
	buffer: &'static mut [u8],
}

impl RxToken {
	pub fn new(buffer: &'static mut [u8]) -> Self {
		Self { buffer }
	}
}

impl phy::RxToken for RxToken {
	#[allow(unused_mut)]
	fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> smoltcp::Result<R>
	where
		F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
	{
		let result = f(self.buffer);
		if unsafe { sys_rx_buffer_consumed().is_ok() } {
			result
		} else {
			Err(smoltcp::Error::Exhausted)
		}
	}
}

#[doc(hidden)]
pub struct TxToken;

impl TxToken {
	pub fn new() -> Self {
		Self {}
	}
}

impl phy::TxToken for TxToken {
	fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
	where
		F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
	{
		let (tx_buffer, handle) =
			unsafe { sys_get_tx_buffer(len).map_err(|_| smoltcp::Error::Exhausted)? };
		let tx_slice: &'static mut [u8] = unsafe { slice::from_raw_parts_mut(tx_buffer, len) };
		match f(tx_slice) {
			Ok(result) => {
				if unsafe { sys_send_tx_buffer(handle, len).is_ok() } {
					Ok(result)
				} else {
					Err(smoltcp::Error::Exhausted)
				}
			}
			Err(e) => {
				unsafe { sys_free_tx_buffer(handle) };
				Err(e)
			}
		}
	}
}
