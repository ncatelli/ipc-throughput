pub struct Sender<T, SID>
where
    SID: AsRef<str>,
{
    _kind: std::marker::PhantomData<T>,
    _shm: SharedMem<SID>,
    _mqueue: MessageQueue,
}

pub struct Receiver<T, SID>
where
    SID: AsRef<str>,
{
    _kind: std::marker::PhantomData<T>,
    _shm: SharedMem<SID>,
    _mqueue: MessageQueue,
}

struct MessageQueue {
    #[allow(unused)]
    queue_name: std::ffi::CString,
    descriptor: libc::mqd_t,
}

impl MessageQueue {
    #[allow(unused)]
    fn try_new<ID: AsRef<str>>(queue_name: ID) -> Option<Self> {
        let name = std::ffi::CString::new(queue_name.as_ref()).ok()?;

        let open_rv = unsafe {
            libc::mq_open(
                name.as_c_str().as_ptr(),
                libc::O_WRONLY | libc::O_CREAT,
                0o600,
                std::ptr::null::<libc::mq_attr>(),
            )
        };

        match open_rv {
            -1 => None,
            descriptor => Some(Self {
                queue_name: name,
                descriptor,
            }),
        }
    }
}

impl Drop for MessageQueue {
    fn drop(&mut self) {
        unsafe { libc::mq_close(self.descriptor) };
    }
}

struct SharedMem<ID: AsRef<str>> {
    _shm_id: ID,
}

impl<ID: AsRef<str>> SharedMem<ID> {
    #[allow(unused)]
    fn try_new(shm_id: ID) -> Self {
        Self { _shm_id: shm_id }
    }
}
