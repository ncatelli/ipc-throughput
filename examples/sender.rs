use std::ffi::CString;

const MSG_SIZE: usize = 8192;

fn main() -> Result<(), String> {
    let buf = CString::new("hello world").unwrap();
    let name = CString::new("/test_queue").unwrap();

    let attr_msgsize = i64::try_from(MSG_SIZE).map_err(|e| e.to_string())?;
    let attr = unsafe {
        let mut uninit_attr = std::mem::MaybeUninit::<libc::mq_attr>::uninit();
        let p = uninit_attr.as_mut_ptr();
        (*p).mq_flags = 0;
        (*p).mq_maxmsg = 10;
        (*p).mq_msgsize = attr_msgsize;
        (*p).mq_curmsgs = 0;
        uninit_attr.assume_init()
    };

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
