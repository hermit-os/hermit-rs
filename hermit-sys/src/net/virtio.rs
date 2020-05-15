#![allow(unused)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::mem::size_of;
use std::ptr;
use std::rc::Rc;
use std::slice::from_raw_parts_mut;

use net::NetworkInterface;
use virtio_bindings::bindings::virtio_net::*;
use virtio_bindings::bindings::virtio_net::__virtio16;
use virtio_bindings::bindings::virtio_ring::*;
use x86::io::*;

use smoltcp::iface::{EthernetInterfaceBuilder, NeighborCache, Routes};
use smoltcp::phy::{self, Device, DeviceCapabilities};
use smoltcp::socket::SocketSet;
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};

extern "Rust" {
    fn sys_pci_find_adapter(vendor_id: u16, class_id: u8, subclass_id: u8) -> Option<([u32; 6], u8)>;
    fn sys_page_alloc(sz: usize) -> (u64, u64);
}

const VIRTIO_NET_MTU: usize = 1500;
const VENDOR_ID: u16 = 0x1AF4;
const VIRTIO_NUM_QUEUES: usize = 2;
const PAGE_SIZE: u64 = 0x1000;
const PAGE_BITS: u64 = 12;
const QUEUE_LIMIT: u16 = 256;
const BUFFER_SIZE: usize = 0x2048;
/* NOTE: RX queue is 0, TX queue is 1 - Virtio Std. §5.1.2  */
const TX_NUM: usize = 1;
const RX_NUM: usize = 0;

/// A 32-bit r/o bitmask of the features supported by the host
const VIRTIO_PCI_HOST_FEATURES: u16 = 0;
/// A 32-bit r/w bitmask of features activated by the guest
const VIRTIO_PCI_GUEST_FEATURES: u16 = 4;
/// A 32-bit r/w PFN for the currently selected queue
const VIRTIO_PCI_QUEUE_PFN: u16 = 8;
/// A 16-bit r/o queue size for the currently selected queue
const VIRTIO_PCI_QUEUE_NUM: u16 = 12;
/// A 16-bit r/w queue selector
const VIRTIO_PCI_QUEUE_SEL: u16 = 14;
/// A 16-bit r/w queue notifier
const VIRTIO_PCI_QUEUE_NOTIFY: u16 = 16;
/// An 8-bit device status register.
const VIRTIO_PCI_STATUS: u16 = 18;
/// The remaining space is defined by each driver as the per-driver
/// configuration space
const VIRTIO_PCI_CONFIG_OFF: u16 = 20;

// u32 is used here for ids for padding reasons.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct virtq_used_elem {
	// Index of start of used descriptor chain.
	id: u32,
	// Total length of the descriptor chain which was written to.
	len: u32,
}

// Virtqueue descriptors: 16 bytes.
// These can chain together via "next".
#[repr(C)]
#[derive(Clone, Debug)]
pub struct virtq_desc_raw {
	// Address (guest-physical)
	// possibly optimize: https://rust-lang.github.io/unsafe-code-guidelines/layout/enums.html#layout-of-a-data-carrying-enums-without-a-repr-annotation
	// https://github.com/rust-lang/rust/pull/62514/files box will call destructor when removed.
	// BUT: we dont know buffer size, so T is not sized in Option<Box<T>> --> Box not simply a pointer?? [TODO: verify this! from https://github.com/rust-lang/unsafe-code-guidelines/issues/157#issuecomment-509016096]
	// nice, we have docs on this: https://doc.rust-lang.org/nightly/std/boxed/index.html#memory-layout
	// https://github.com/rust-lang/rust/issues/52976
	// Vec<T> is sized! but not just an array in memory.. --> impossible
	pub addr: u64,
	// Length
	pub len: u32,
	// The flags as indicated above (VIRTQ_DESC_F_*)
	pub flags: u16,
	// next field, if flags & NEXT
	// We chain unused descriptors via this, too
	pub next: u16,
}

impl Drop for virtq_desc_raw {
	fn drop(&mut self) {
		// TODO: what happens on shutdown etc?
		warn!("Dropping virtq_desc_raw, this is likely an error as of now! No memory will be deallocated!");
	}
}

// Single virtq descriptor. Pointer to raw descr, together with index
#[derive(Debug)]
struct VirtqDescriptor {
	index: u16,
	raw: Box<virtq_desc_raw>,
}

#[derive(Debug)]
struct VirtqDescriptorChain(Vec<VirtqDescriptor>);

// Two descriptor chains are equal, if memory address of vec is equal.
impl PartialEq for VirtqDescriptorChain {
	fn eq(&self, other: &Self) -> bool {
		&self.0 as *const _ == &other.0 as *const _
	}
}

struct VirtqDescriptors {
	// We need to guard against mem::forget. --> always store chains here?
	//    Do we? descriptors are in this file only, not external! -> We can ensure they are not mem::forgotten?
	//    still need to have them stored in this file somewhere though, cannot be owned by moved-out transfer object.
	//    So this is best solution?
	// free contains a single chain of all currently free descriptors.
	free: RefCell<VirtqDescriptorChain>,
	// a) We want to be able to use nonmutable reference to create new used chain
	// b) we want to return reference to descriptor chain, eg when creating new!
	// TODO: improve this type. there should be a better way to accomplish something similar.
	used_chains: RefCell<Vec<Rc<RefCell<VirtqDescriptorChain>>>>,
}

struct Virtq<'a> {
	index: u16,  // Index of vq in common config
	vqsize: u16, // Elements in ring/descrs
	// The actial descriptors (16 bytes each)
	virtq_desc: VirtqDescriptors,
	// A ring of available descriptor heads with free-running index
	avail: Rc<RefCell<VirtqAvail<'a>>>,
	// A ring of used descriptor heads with free-running index
	used: Rc<RefCell<VirtqUsed<'a>>>,
	// Address where queue index is written to on notify
	queue_notify_address: &'a mut u16,
}

#[allow(dead_code)]
struct VirtqAvail<'a> {
	flags: &'a mut u16, // If VIRTIO_F_EVENT_IDX, set to 1 to maybe suppress interrupts
	idx: &'a mut u16,
	ring: &'a mut [u16],
	//rawmem: Box<[u16]>,
	// Only if VIRTIO_F_EVENT_IDX used_event: u16,
}

#[allow(dead_code)]
struct VirtqUsed<'a> {
	flags: &'a u16,
	idx: &'a u16,
	ring: &'a [virtq_used_elem],
	//rawmem: Box<[u16]>,
	last_idx: u16,
}

#[derive(Debug, Default)]
#[repr(C)]
struct VirtQueue {
	ring: vring,
	virt_buffer: u64,
	phys_buffer: u64,
	last_seen_used: u64,
}

/// Data type to determine the mac address
#[derive(Debug, Default)]
#[repr(C)]
pub struct VirtioNet {
    queues: [VirtQueue; VIRTIO_NUM_QUEUES]
}

unsafe impl Send for VirtioNet {}

unsafe fn __is_network_available() -> bool {
    let (base_addresses, irq) = sys_pci_find_adapter(VENDOR_ID, 0x02 /* Network */, 0x00 /* Ethernet */).unwrap_or(([0; 6], 0));
    if base_addresses[0] == 0 {
        return false;
    }
    
    let iobase: u16 = base_addresses[0].try_into().unwrap();
    let iomem: u32 = base_addresses[1];

    info!("Virtio-Net uses IRQ {}, IO port 0x{:x}, and IO men 0x{:x}", irq, iobase, iomem);

    // reset interface
    outb(iobase + VIRTIO_PCI_STATUS, 0);
    info!("Virtio-Net status: 0x{:x}", inb(iobase + VIRTIO_PCI_STATUS));

    // tell the device that we have noticed it
    outb(iobase + VIRTIO_PCI_STATUS, VIRTIO_CONFIG_S_ACKNOWLEDGE as u8);
    // tell the device that we will support it.
    outb(iobase + VIRTIO_PCI_STATUS, (VIRTIO_CONFIG_S_ACKNOWLEDGE|VIRTIO_CONFIG_S_DRIVER) as u8);

    trace!("Host features 0x{:x}", inl(iobase + VIRTIO_PCI_HOST_FEATURES));

    let features = inl(iobase + VIRTIO_PCI_HOST_FEATURES);
    let required = (1 << VIRTIO_NET_F_MAC) | (1 << VIRTIO_NET_F_STATUS);

    if (features & required) != required {
    	error!("Host isn't able to fulfill HermitCore's requirements\n");
        outb(iobase + VIRTIO_PCI_STATUS, VIRTIO_CONFIG_S_FAILED as u8);

    	return false;
    }

    let mut required = features;
    required &= !(1 << VIRTIO_NET_F_CTRL_VQ);
    required &= !(1 << VIRTIO_NET_F_GUEST_TSO4);
    required &= !(1 << VIRTIO_NET_F_GUEST_TSO6);
    required &= !(1 << VIRTIO_NET_F_GUEST_UFO);
    required &= !(1 << VIRTIO_RING_F_EVENT_IDX);
    required &= !(1 << VIRTIO_NET_F_MRG_RXBUF);
    required &= !(1 << VIRTIO_NET_F_MQ);

    trace!("wanted guest features 0x{:x}", required);
	outl(iobase + VIRTIO_PCI_GUEST_FEATURES, required);
	let features = inl(iobase + VIRTIO_PCI_GUEST_FEATURES);
    trace!("current guest features 0x{:x}", features);
    
    // tell the device that the features are OK
    outb(
    	iobase + VIRTIO_PCI_STATUS,
       	(VIRTIO_CONFIG_S_ACKNOWLEDGE|VIRTIO_CONFIG_S_DRIVER|VIRTIO_CONFIG_S_FEATURES_OK) as u8
    );

	// check if the host accept these features
    let status = inb(iobase + VIRTIO_PCI_STATUS);
	if (status as u32 & VIRTIO_CONFIG_S_FEATURES_OK) != VIRTIO_CONFIG_S_FEATURES_OK {
	    error!("device features are ignored: status 0x{:x}", status);
        outb(iobase + VIRTIO_PCI_STATUS, VIRTIO_CONFIG_S_FAILED as u8);

	    false
	} else {
        true
    }
}

pub fn is_network_available() -> bool {
    unsafe {
        __is_network_available()
    }
}

fn vring_size(num: usize, align: usize) -> usize {
    ((size_of::<vring_desc>() * num + size_of::<__virtio16>() * (3 + num) + align - 1) & !(align - 1))
		+ size_of::<__virtio16>() * 3 + size_of::<vring_used_elem>() * num
}

unsafe fn vring_init(ring: &mut vring, num: usize, p: usize, align: usize) {
    ring.num = num.try_into().unwrap();
    ring.desc = p as *mut virtio_bindings::bindings::virtio_ring::vring_desc;
    ring.avail = (p as usize + num * size_of::<vring_desc>()) as *mut virtio_bindings::bindings::virtio_ring::vring_avail;
    let avail = ring.avail;
    ring.used = ((&((*avail).ring.as_slice(num+1)[num]) as *const _ as usize + size_of::<__virtio16>() + align-1) & !(align - 1)) as *mut virtio_bindings::bindings::virtio_ring::vring_used;
}

impl NetworkInterface<VirtioNet> {
	pub fn new() -> Option<Self> {
        let (base_addresses, irq) = unsafe { sys_pci_find_adapter(VENDOR_ID, 0x02 /* Network */, 0x00 /* Ethernet */).unwrap_or(([0; 6], 0)) };
        if base_addresses[0] == 0 {
            return None;
        }
    
        let iobase: u16 = base_addresses[0].try_into().unwrap();

        // determine the mac address of this card
        let mut mac: [u8; 6] = [0; 6];
	    for i in 0..6 {
            mac[i] = unsafe { inb(iobase + VIRTIO_PCI_CONFIG_OFF + i as u16) };
        }

        let mut device = VirtioNet::default();

        // Setup virt queues
        for index in 0..VIRTIO_NUM_QUEUES {
            unsafe {
                // determine queue size
		        outw(iobase+VIRTIO_PCI_QUEUE_SEL, index.try_into().unwrap());
		        let mut num = inw(iobase+VIRTIO_PCI_QUEUE_NUM);
		        if num == 0 {
                    error!("Invalid queue size");
                    return None;
                }

                info!("Virtio-Net: queue_size {} (index {})", num, index);

                let total_size = vring_size(num as usize, PAGE_SIZE as usize);

                // allocate and init memory for the virtual queue
                let (virt_addr, phys_addr) = sys_page_alloc(total_size);
                ptr::write_bytes(virt_addr as *mut u8, 0x00, total_size);

                unsafe {
                    vring_init(&mut device.queues[index].ring, num as usize, virt_addr as usize, PAGE_SIZE as usize);
                }

                if num > QUEUE_LIMIT {
                    device.queues[index].ring.num = QUEUE_LIMIT as u32;
                    num = QUEUE_LIMIT;
                    info!("Virtio-Net: set queue limit to {} (index {})\n", num, index);
                }

                let (virt_buffer, phys_buffer) = sys_page_alloc(BUFFER_SIZE * num as usize);
                device.queues[index].virt_buffer = virt_buffer;
                device.queues[index].phys_buffer = phys_buffer;
        
                for i in 0..num {
                    from_raw_parts_mut(device.queues[index].ring.desc, num as usize+1)[i as usize].addr = phys_buffer + i as u64 * BUFFER_SIZE as u64;
			        if index == RX_NUM {
                        /* NOTE: RX queue is 0, TX queue is 1 - Virtio Std. §5.1.2  */
                        let desc =  from_raw_parts_mut(device.queues[index].ring.desc, num as usize);
                        desc[i as usize].len = BUFFER_SIZE as u32;
                        desc[i as usize].flags = VRING_DESC_F_WRITE.try_into().unwrap();
                        let ring = &mut device.queues[index].ring;
                        let idx = (*ring.avail).idx;
                        (*ring.avail).ring.as_mut_slice(num as usize)[(idx % num) as usize] = i;
                        (*ring.avail).idx = idx + 1;
			        }
                }

                // register buffer
		        outw(iobase+VIRTIO_PCI_QUEUE_SEL, index as u16);
		        outl(iobase+VIRTIO_PCI_QUEUE_PFN, (phys_addr >> PAGE_BITS).try_into().unwrap());
            }
        }
    
        // tell the device that the drivers is initialized
	    unsafe {
            outb(iobase + VIRTIO_PCI_STATUS, (VIRTIO_CONFIG_S_ACKNOWLEDGE|VIRTIO_CONFIG_S_DRIVER|VIRTIO_CONFIG_S_DRIVER_OK|VIRTIO_CONFIG_S_FEATURES_OK) as u8);
        }

        info!("Virtio-Net status: 0x{:x}", unsafe { inb(iobase + VIRTIO_PCI_STATUS) });
        if unsafe { inl(iobase + VIRTIO_PCI_CONFIG_OFF + 6) } as u32 & VIRTIO_NET_S_LINK_UP == VIRTIO_NET_S_LINK_UP { 
            info!("Virtio-Net link is up");
        } else {
            info!("Virtio-Net link is down");
        }

        let myip: [u8; 4] = [10,0,5,3];
        let mygw: [u8; 4] = [10,0,5,1];
		let mymask: [u8; 4] = [255,255,255,0];

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
		info!("Configure gatway with address {}", default_v4_gw);

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

impl<'a> Device<'a> for VirtioNet {
	type RxToken = RxToken;
	type TxToken = TxToken;

	fn capabilities(&self) -> DeviceCapabilities {
		let mut cap = DeviceCapabilities::default();
		cap.max_transmission_unit = VIRTIO_NET_MTU;
		cap
	}

	fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
		/*let rx_queue = unsafe { &mut *(SHAREDQUEUE_START as *mut u8 as *mut SharedQueue) };

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
        }*/
        None
	}

	fn transmit(&'a mut self) -> Option<Self::TxToken> {
		trace!("create TxToken to transfer data");
        //Some(TxToken::new())
        None
	}
}

#[doc(hidden)]
pub struct RxToken {
	buffer: [u8; VIRTIO_NET_MTU],
	len: u16,
}

impl RxToken {
	pub fn new(len: u16) -> RxToken {
		RxToken {
			buffer: [0; VIRTIO_NET_MTU],
			len: len,
		}
	}
}

impl phy::RxToken for RxToken {
	fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> smoltcp::Result<R>
	where
		F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
	{
		/*let (first, _) = self.buffer.split_at_mut(self.len as usize);
        f(first)*/
        Err(smoltcp::Error::Dropped)
	}
}

#[doc(hidden)]
pub struct TxToken {
    virt_buffer: u64,
}

impl TxToken {
	pub const fn new(virt_buffer: u64) -> Self {
		TxToken {
            virt_buffer: virt_buffer,
        }
	}
}

impl phy::TxToken for TxToken {
	fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
	where
		F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
	{
        /*for(buffer_index=0; buffer_index<vq->vring.num; buffer_index++) {
            if (!vq->vring.desc[buffer_index].len) {
                LOG_DEBUG("vioif_output: buffer %u is free\n", buffer_index);
                break;
            }
        }

        let hdr_sz = size_of::<virtio_net_hdr>();*/
        // NOTE: packet is fully checksummed => all flags are set to zero
        //ptr::write_bytes((self.virt_buffer + buffer_index * BUFFER_SIZE) as *mut u8, 0x00, hdr_sz);
    
		/*let tx_queue = unsafe {
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
        }*/
        Err(smoltcp::Error::Dropped)
	}
}
