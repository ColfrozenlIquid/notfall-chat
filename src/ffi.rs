use std::{
    ffi::{CString, c_int},
    os::raw::c_char,
};

pub const DEVICE_NAME_LEN: usize = 16;
pub const DEVICE_ADDR_LEN: usize = 46;
pub const DISCOVERY_NAME_LEN: usize = 32;
pub const INET_ADDRSTRLEN: usize = 16;

pub enum Connection {}

#[derive(Clone)]
pub struct ConnectionHandle(*mut Connection);

unsafe impl Send for ConnectionHandle {}

impl ConnectionHandle {
    pub fn connect(ip: &str, port: u16) -> Result<Self, String> {
        let conn = unsafe { connection_create() };
        if conn.is_null() {
            return Err("allocation failed".into());
        }
        let ip = CString::new(ip).map_err(|e| e.to_string())?;
        let ret = unsafe { connect_to_server(ip.as_ptr(), port as c_int, conn) };
        if ret == 0 {
            Ok(Self(conn))
        } else {
            unsafe { connection_destroy(conn) };
            Err(format!("connect_to_server returned {ret}"))
        }
    }

    pub fn send(&self, msg: &str) -> Result<(), String> {
        let bytes = msg.as_bytes();
        let ret = unsafe { connection_send(self.0, bytes.as_ptr(), bytes.len()) };
        if ret == 0 {
            Ok(())
        } else {
            Err(format!("send failed: {ret}"))
        }
    }

    pub fn wait(self) {
        unsafe {
            connection_wait(self.0);
        }
    }
}

impl Drop for ConnectionHandle {
    fn drop(&mut self) {
        unsafe {
            connection_destroy(self.0);
        }
    }
}

#[repr(C)]
pub struct C_NetworkDevice {
    pub name: [c_char; DEVICE_NAME_LEN],
    pub addr: [c_char; DEVICE_ADDR_LEN],
    pub is_ipv6: i32,
}

#[repr(C)]
pub struct C_DiscoveredPeer {
    pub timestamp: u64,
    pub port: u16,
    pub sender_ip: [c_char; INET_ADDRSTRLEN],
    pub name: [u8; DISCOVERY_NAME_LEN],
}

unsafe extern "C" {
    pub fn get_network_devices(devices: *mut C_NetworkDevice, len: usize) -> usize;
    pub fn broadcast_discovery(name: *mut u8, name_len: usize, tcp_port: u16) -> i32;
    pub fn discovery_listener_start();
    pub fn discovery_listener_pop(out: *mut C_DiscoveredPeer) -> i32;
    pub fn run_server(port: i32) -> i32;

    pub fn connection_create() -> *mut Connection;
    pub fn connection_destroy(conn: *mut Connection);
    pub fn connect_to_server(ip: *const c_char, port: i32, conn: *mut Connection) -> i32;
    pub fn connection_send(conn: *mut Connection, data: *const u8, len: usize) -> i32;
    pub fn connection_wait(conn: *mut Connection);
}
