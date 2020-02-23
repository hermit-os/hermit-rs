/// Just a few dummy functions if smoltcp support is disabled

// A handle, identifying a socket
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Handle(usize);

#[no_mangle]
pub fn sys_socket_connect(_ip: &[u8], _port: u16, _timeout: Option<u64>) -> Result<Handle, ()> {
	Err(())
}

#[no_mangle]
pub fn sys_socket_read(_handle: Handle, _buffer: &mut [u8]) -> Result<usize, ()> {
	Err(())
}

#[no_mangle]
pub fn sys_socket_write(_handle: Handle, _buffer: &[u8]) -> Result<usize, ()> {
	Err(())
}

#[no_mangle]
pub fn sys_socket_close(_handle: Handle) -> Result<(), ()> {
	Err(())
}
