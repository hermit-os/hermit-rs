use std::collections::BTreeMap;
#[cfg(not(feature = "dhcpv4"))]
use std::net::Ipv4Addr;
use std::slice;

#[cfg(feature = "dhcpv4")]
use smoltcp::dhcp::Dhcpv4Client;
use smoltcp::iface::{EthernetInterfaceBuilder, NeighborCache, Routes};
#[cfg(feature = "trace")]
use smoltcp::phy::EthernetTracer;
use smoltcp::phy::{self, Device, DeviceCapabilities};
use smoltcp::socket::SocketSet;
#[cfg(feature = "dhcpv4")]
use smoltcp::socket::{RawPacketMetadata, RawSocketBuffer};
use smoltcp::time::Instant;
#[cfg(not(feature = "dhcpv4"))]
use smoltcp::wire::IpAddress;
#[cfg(feature = "dhcpv4")]
use smoltcp::wire::Ipv4Cidr;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address};

use crate::net::waker::WakerRegistration;
use crate::net::{NetworkInterface, NetworkState};

extern "Rust" {
	fn sys_get_mac_address() -> Result<[u8; 6], ()>;
	fn sys_get_mtu() -> Result<u16, ()>;
	fn sys_get_tx_buffer(len: usize) -> Result<(*mut u8, usize), ()>;
	fn sys_send_tx_buffer(handle: usize, len: usize) -> Result<(), ()>;
	fn sys_receive_rx_buffer() -> Result<(&'static mut [u8], usize), ()>;
	fn sys_rx_buffer_consumed(handle: usize) -> Result<(), ()>;
	fn sys_free_tx_buffer(handle: usize);
}

/// Data type to determine the mac address
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub(crate) struct HermitNet {
	pub mtu: u16,
}

impl HermitNet {
	pub(crate) const fn new(mtu: u16) -> Self {
		Self { mtu }
	}
}

/// Returns the value of the specified environment variable.
///
/// The value is fetched from the current runtime environment and, if not
/// present, falls back to the same environment variable set at compile time
/// (might not be present as well).
///
/// # Panics
///
/// Panics when environment variable is not valid unicode.
macro_rules! hermit_var {
	($name:expr) => {{
		use std::borrow::Cow;
		use std::env::{self, VarError};

		match env::var($name) {
			Ok(val) => Some(Cow::Owned(val)),
			Err(VarError::NotPresent) => option_env!($name).map(Cow::Borrowed),
			Err(VarError::NotUnicode(s)) => {
				panic!("couldn't interpret {}: {}", $name, VarError::NotUnicode(s))
			}
		}
	}};
}

/// Tries to parse the specified environment variable with a default value.
///
/// Parses according to [`hermit_var`] or returns the specified default value.
///
/// # Panics
///
/// Panics when environment variable is not valid unicode or cannot be parsed.
macro_rules! parse_hermit_var_or {
	($name:expr, $default:expr) => {{
		hermit_var!($name)
			.map(|ip| {
				ip.parse()
					.unwrap_or_else(|err| panic!("{err}: {}: {ip}", $name))
			})
			.unwrap_or($default)
	}};
}

impl NetworkInterface<HermitNet> {
	#[cfg(feature = "dhcpv4")]
	pub(crate) fn new() -> NetworkState {
		let mtu = match unsafe { sys_get_mtu() } {
			Ok(mtu) => mtu,
			Err(_) => {
				return NetworkState::InitializationFailed;
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
				return NetworkState::InitializationFailed;
			}
		};

		let neighbor_cache = NeighborCache::new(BTreeMap::new());
		let ethernet_addr = EthernetAddress([mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]]);
		let ip_addrs = [IpCidr::new(Ipv4Address::UNSPECIFIED.into(), 0)];
		let routes = Routes::new(BTreeMap::new());

		info!("MAC address {}", ethernet_addr);
		info!("MTU: {} bytes", mtu);

		let mut sockets = SocketSet::new(vec![]);
		let dhcp_rx_buffer = RawSocketBuffer::new([RawPacketMetadata::EMPTY; 1], vec![0; 900]);
		let dhcp_tx_buffer = RawSocketBuffer::new([RawPacketMetadata::EMPTY; 1], vec![0; 600]);
		let dhcp = Dhcpv4Client::new(&mut sockets, dhcp_rx_buffer, dhcp_tx_buffer, Instant::now());
		let prev_cidr = Ipv4Cidr::new(Ipv4Address::UNSPECIFIED, 0);

		let iface = EthernetInterfaceBuilder::new(device)
			.ethernet_addr(ethernet_addr)
			.neighbor_cache(neighbor_cache)
			.ip_addrs(ip_addrs)
			.routes(routes)
			.finalize();

		NetworkState::Initialized(Self {
			iface,
			sockets,
			dhcp,
			prev_cidr,
			waker: WakerRegistration::new(),
		})
	}

	#[cfg(not(feature = "dhcpv4"))]
	pub(crate) fn new() -> NetworkState {
		let mtu = match unsafe { sys_get_mtu() } {
			Ok(mtu) => mtu,
			Err(_) => {
				return NetworkState::InitializationFailed;
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
				return NetworkState::InitializationFailed;
			}
		};

		let myip = parse_hermit_var_or!("HERMIT_IP", Ipv4Addr::new(10, 0, 5, 3));
		let myip = myip.octets();
		let mygw = parse_hermit_var_or!("HERMIT_GATEWAY", Ipv4Addr::new(10, 0, 5, 1));
		let mygw = mygw.octets();
		let mymask = parse_hermit_var_or!("HERMIT_MASK", Ipv4Addr::new(255, 255, 255, 0));
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

		NetworkState::Initialized(Self {
			iface,
			sockets: SocketSet::new(vec![]),
			waker: WakerRegistration::new(),
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
			Ok((buffer, handle)) => Some((RxToken::new(buffer, handle), TxToken::new())),
			_ => None,
		}
	}

	fn transmit(&'a mut self) -> Option<Self::TxToken> {
		trace!("create TxToken to transfer data");
		Some(TxToken::new())
	}
}

#[doc(hidden)]
pub(crate) struct RxToken {
	buffer: &'static mut [u8],
	handle: usize,
}

impl RxToken {
	pub(crate) fn new(buffer: &'static mut [u8], handle: usize) -> Self {
		Self { buffer, handle }
	}
}

impl phy::RxToken for RxToken {
	#[allow(unused_mut)]
	fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> smoltcp::Result<R>
	where
		F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
	{
		let result = f(self.buffer);
		if unsafe { sys_rx_buffer_consumed(self.handle).is_ok() } {
			result
		} else {
			Err(smoltcp::Error::Exhausted)
		}
	}
}

#[doc(hidden)]
pub(crate) struct TxToken;

impl TxToken {
	pub(crate) fn new() -> Self {
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
