use std::io::Write;

use ipc_throughput::*;

fn main() -> Result<(), String> {
    let buf = "hello world";
    let attr_msgsize = i64::try_from(buf.data_size()).map_err(|e| e.to_string())?;
    let mut mq = MessageQueue::<&str>::try_new_with_attr(
        "/test_queue",
        MQueueFlags::new(MQueueMode::WriteOnly).with_option(MQueueOption::Create),
        MQueueAttr::new(0, 10, attr_msgsize, 0),
    )
    .ok_or_else(|| "mq_open error".to_string())?;

    mq.write(buf.encode())
        .map_err(|e| e.to_string())
        .map(|_| ())
}
