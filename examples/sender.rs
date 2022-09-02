use std::ffi::CString;

fn main() -> Result<(), String> {
    let buf = CString::new("hello world").unwrap();
    let name = CString::new("/test_queue").unwrap();

    let attr_msgsize = i64::try_from(buf.as_bytes_with_nul().len()).map_err(|e| e.to_string())?;
    let attr = ipc_throughput::Attr::new(0, 10, attr_msgsize, 0).to_mq_attr();

    let mq = unsafe {
        libc::mq_open(
            name.as_c_str().as_ptr(),
            libc::O_WRONLY | libc::O_CREAT,
            0o600,
            &attr,
        )
    };

    if mq == -1_isize as libc::mqd_t {
        return Err("mq_open error".to_string());
    } else {
        let rv = unsafe { libc::mq_send(mq, buf.as_ptr(), buf.as_bytes_with_nul().len(), 0) };
        if rv != 0 {
            return Err("mq_send error".to_string());
        }
    }

    unsafe { libc::mq_close(mq) };

    Ok(())
}
