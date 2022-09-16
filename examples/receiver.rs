use std::ffi::CStr;
use std::io::Read;

use ipc_throughput::*;

const MSG_SIZE: usize = 8196;

fn main() -> Result<(), String> {
    let mut buf: [u8; MSG_SIZE] = [0; MSG_SIZE];

    let shm = SharedMem::<&str, 10>::try_new(
        "test",
        SharedMemFlags::new(SharedMemMode::ReadWrite).with_option(SharedMemOption::Create),
    )
    .ok_or_else(|| "shm_open error".to_string())?;

    let mut receiver =
        bounded_sync_receiver(&shm).ok_or_else(|| "unable to instantiate sender".to_string())?;

    let read_bytes = receiver.read(&mut buf).map_err(|e| e.to_string())?;

    let buf_str = unsafe { CStr::from_ptr(buf.as_ptr() as *const i8) }
        .to_str()
        .map_err(|e| e.to_string())?;

    println!("read: {}\n{}", read_bytes, buf_str);

    Ok(())
}
