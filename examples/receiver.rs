use std::ffi::{CStr, CString};

const MSG_SIZE: usize = 4096;

fn main() -> Result<(), String> {
    let mut buf: [libc::c_char; MSG_SIZE] = [0; MSG_SIZE];
    let name = CString::new("/test_queue").unwrap();

    let attr_msgsize = i64::try_from(MSG_SIZE).map_err(|e| e.to_string())?;
    let attr = ipc_throughput::MQueueAttr::new(0, 10, attr_msgsize, 0).to_mq_attr();

    let mq = unsafe { libc::mq_open(name.as_ptr(), libc::O_RDWR | libc::O_CREAT, 0o600, &attr) };

    if mq == -1_isize as libc::mqd_t {
        return Err("mq_open error".to_string());
    }

    let read_bytes =
        unsafe { libc::mq_receive(mq, buf.as_mut_ptr(), 8192, std::ptr::null_mut::<u32>()) };
    if read_bytes == -1_isize {
        return Err("mq_receive error".to_string());
    }

    let buf_str = unsafe { CStr::from_ptr(buf.as_ptr()) }
        .to_str()
        .map_err(|e| e.to_string())?;

    println!("read: {}\n{}", read_bytes, buf_str);

    unsafe {
        libc::mq_close(mq);
        libc::mq_unlink(name.as_c_str().as_ptr())
    };

    Ok(())
}
