use std::io::Write;

use ipc_throughput::*;

fn main() -> Result<(), String> {
    let buf = "hello world";

    let shm = SharedMem::<&str, 10>::try_new(
        "test",
        SharedMemFlags::new(SharedMemMode::ReadWrite).with_option(SharedMemOption::Create),
    )
    .ok_or_else(|| "shm_open error".to_string())?;

    let mut sender =
        bounded_sync_sender(&shm).ok_or_else(|| "unable to instantiate sender".to_string())?;

    sender
        .write(buf.encode())
        .map_err(|e| e.to_string())
        .map(|_| ())
}
