pub struct Attr {
    flags: i64,
    max_msg: i64,
    msg_size: i64,
    cur_msgs: i64,
}

impl Attr {
    pub fn new(flags: i64, max_msg: i64, msg_size: i64, cur_msgs: i64) -> Self {
        Self {
            flags,
            max_msg,
            msg_size,
            cur_msgs,
        }
    }

    pub fn try_new_with_sized_type<T>(flags: i64, max_msg: i64, cur_msgs: i64) -> Option<Self> {
        let size_of_t = std::mem::size_of::<T>();
        let attr_msgsize = i64::try_from(size_of_t).ok()?;

        Some(Self {
            flags,
            max_msg,
            msg_size: attr_msgsize,
            cur_msgs,
        })
    }

    pub fn to_mq_attr(&self) -> libc::mq_attr {
        unsafe {
            let mut uninit_attr = std::mem::MaybeUninit::<libc::mq_attr>::uninit();
            let p = uninit_attr.as_mut_ptr();
            (*p).mq_flags = self.flags;
            (*p).mq_maxmsg = self.max_msg;
            (*p).mq_msgsize = self.msg_size;
            (*p).mq_curmsgs = self.cur_msgs;
            uninit_attr.assume_init()
        }
    }
}

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
