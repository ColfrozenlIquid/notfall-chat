use std::{
    ffi::{CString, c_int},
    os::raw::c_char,
    sync::Arc,
};

pub const DEVICE_NAME_LEN: usize = 16;
pub const DEVICE_ADDR_LEN: usize = 46;
pub const DISCOVERY_NAME_LEN: usize = 32;
pub const INET_ADDRSTRLEN: usize = 16;

pub enum Connection {}

pub type OnAcceptCb = unsafe extern "C" fn(*mut Connection, *mut std::ffi::c_void);

#[derive(Clone)]
pub struct ConnectionHandle(Arc<ConnectionInner>);

struct ConnectionInner(*mut Connection);

unsafe impl Send for ConnectionHandle {}

impl Drop for ConnectionInner {
    fn drop(&mut self) {
        unsafe { connection_destroy(self.0) };
    }
}

unsafe impl Send for ConnectionInner {}
unsafe impl Sync for ConnectionInner {}

impl ConnectionHandle {
    pub fn connect(ip: &str, port: u16) -> Result<Self, String> {
        let conn = unsafe { connection_create() };
        if conn.is_null() {
            return Err("allocation failed".into());
        }
        let ip = CString::new(ip).map_err(|e| e.to_string())?;
        let ret = unsafe { connect_to_server(ip.as_ptr(), port as c_int, conn) };
        if ret == 0 {
            Ok(Self(Arc::new(ConnectionInner(conn))))
        } else {
            unsafe { connection_destroy(conn) };
            Err(format!("connect_to_server returned {ret}"))
        }
    }

    pub fn send(&self, msg: &str) -> Result<(), String> {
        let bytes = msg.as_bytes();
        let ret = unsafe { connection_send(self.0.0, bytes.as_ptr(), bytes.len()) };
        if ret == 0 {
            Ok(())
        } else {
            Err(format!("send failed: {ret}"))
        }
    }

    pub fn wait(self) {
        unsafe {
            connection_wait(self.0.0);
        }
    }

    pub fn receive(&self) -> Result<String, String> {
        let mut buf = vec![0u8; 8192];
        let mut len: usize = 0;
        let ret = unsafe { connection_receive(self.0.0, buf.as_mut_ptr(), &mut len) };
        match ret {
            1 => {
                buf.truncate(len);
                Ok(String::from_utf8(buf).map_err(|e| e.to_string())?)
            }
            _ => Err(format!("recv failed: {ret}")),
        }
    }

    pub fn try_receive(&self) -> Result<Option<String>, String> {
        let mut buf = vec![0u8; 8192];
        let mut len: usize = 0;
        let ret = unsafe { connection_try_receive(self.0.0, buf.as_mut_ptr(), &mut len) };
        match ret {
            0 => Ok(None),
            1 => {
                buf.truncate(len);
                Ok(Some(String::from_utf8(buf).map_err(|e| e.to_string())?))
            }
            _ => Err(format!("recv failed: {ret}")),
        }
    }

    pub fn print_loss_rate(&self) {
        let ret = unsafe { connection_loss_rate(self.0.0) };
        println!("Connection loss rate: {}", ret * 100);
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
    pub fn run_server(port: i32, cb: OnAcceptCb, userdata: *mut std::ffi::c_void) -> i32;

    pub fn connection_create() -> *mut Connection;
    pub fn connection_destroy(conn: *mut Connection);
    pub fn connect_to_server(ip: *const c_char, port: i32, conn: *mut Connection) -> i32;
    pub fn connection_send(conn: *mut Connection, data: *const u8, len: usize) -> i32;
    pub fn connection_wait(conn: *mut Connection);
    pub fn connection_receive(conn: *mut Connection, dst: *mut u8, out_len: *mut usize) -> i32;
    pub fn connection_try_receive(conn: *mut Connection, dst: *mut u8, out_len: *mut usize) -> i32;

    pub fn connection_srtt(conn: *mut Connection) -> f64;
    pub fn connection_rttvar(conn: *mut Connection) -> f64;
    pub fn connection_sent(conn: *mut Connection) -> u32;
    pub fn connection_lost(conn: *mut Connection) -> u32;
    pub fn connection_loss_rate(conn: *mut Connection) -> f64;
}

pub unsafe extern "C" fn on_accept_callback(
    conn: *mut Connection,
    userdata: *mut std::ffi::c_void,
) {
    let sender = unsafe { &*(userdata as *const std::sync::mpsc::Sender<ConnectionHandle>) };
    let handle = ConnectionHandle(Arc::new(ConnectionInner(conn)));
    let _ = sender.send(handle);
}
