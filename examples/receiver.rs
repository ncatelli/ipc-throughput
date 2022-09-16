use std::ffi::CStr;
use std::io::Read;

use ipc_throughput::*;

const MSG_SIZE: usize = 8196;

fn main() -> Result<(), String> {
    let mut buf: [u8; MSG_SIZE] = [0; MSG_SIZE];

    let attr_msgsize = i64::try_from(<[u8; MSG_SIZE]>::data_size()).map_err(|e| e.to_string())?;
    let mut mq = MessageQueue::<&str>::try_new_with_attr(
        "/test",
        MQueueFlags::new(MQueueMode::ReadOnly).with_option(MQueueOption::Create),
        MQueueAttr::new(0, 10, attr_msgsize, 0),
    )
    .ok_or_else(|| "mq_open error".to_string())?;

    let read_bytes = mq.read(&mut buf).map_err(|e| e.to_string())?;

    let buf_str = unsafe { CStr::from_ptr(buf.as_ptr() as *const i8) }
        .to_str()
        .map_err(|e| e.to_string())?;

    println!("read: {}\n{}", read_bytes, buf_str);

    Ok(())
}
