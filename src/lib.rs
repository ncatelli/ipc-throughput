use std::cell::RefCell;
use std::sync::Arc;

pub struct MQueueAttr {
    flags: i64,
    max_msg: i64,
    msg_size: i64,
    cur_msgs: i64,
}

impl MQueueAttr {
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

pub struct Sender<T: Sendable> {
    _shm: Arc<SharedMem<T>>,
    mqueue: Arc<RefCell<MessageQueue<T>>>,
}

impl<T: Sendable> Sender<T> {
    fn new(_shm: Arc<SharedMem<T>>, mqueue: Arc<RefCell<MessageQueue<T>>>) -> Self {
        Self { _shm, mqueue }
    }
}

impl<T> std::io::Write for Sender<T>
where
    T: Sendable,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut queue = self.mqueue.as_ref().borrow_mut();
        queue.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct Receiver<T: Sendable> {
    _shm: Arc<SharedMem<T>>,
    mqueue: Arc<RefCell<MessageQueue<T>>>,
}

impl<T> std::io::Read for Receiver<T>
where
    T: Sendable,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut queue = self.mqueue.as_ref().borrow_mut();
        queue.read(buf)
    }
}

#[repr(i32)]
pub enum MQueueMode {
    ReadOnly = libc::O_RDONLY,
    WriteOnly = libc::O_WRONLY,
    ReadWrite = libc::O_RDWR,
}

#[repr(i32)]
pub enum MQueueOption {
    CloseOnExec = libc::O_CLOEXEC,
    Create = libc::O_CREAT,
    Exclusive = libc::O_EXCL,
    NonBlocking = libc::O_NONBLOCK,
}

#[derive(Default)]
pub struct MQueueFlags(libc::c_int);

impl MQueueFlags {
    pub fn new(mode: MQueueMode) -> Self {
        Self(mode as i32)
    }

    pub fn with_option(self, option: MQueueOption) -> Self {
        let new_flags = self.0 | (option as i32);
        Self(new_flags)
    }
}

impl MQueueFlags {
    pub fn as_i32(&self) -> i32 {
        self.0
    }

    pub fn as_c_int(&self) -> libc::c_int {
        self.0
    }
}

pub trait Sendable {
    fn data_size(&self) -> usize;
    fn encode(&self) -> &[u8];
}

impl Sendable for std::ffi::CStr {
    fn data_size(&self) -> usize {
        self.to_bytes_with_nul().len()
    }

    fn encode(&self) -> &[u8] {
        self.to_bytes_with_nul()
    }
}

impl Sendable for std::ffi::CString {
    fn data_size(&self) -> usize {
        self.as_c_str().data_size()
    }

    fn encode(&self) -> &[u8] {
        self.as_c_str().encode()
    }
}

impl Sendable for &[u8] {
    fn data_size(&self) -> usize {
        self.len()
    }

    fn encode(&self) -> &[u8] {
        self
    }
}

impl Sendable for &str {
    fn data_size(&self) -> usize {
        self.as_bytes().len()
    }

    fn encode(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Sendable for String {
    fn data_size(&self) -> usize {
        self.as_bytes().len()
    }

    fn encode(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[derive(Clone)]
pub struct MessageQueue<T: Sendable> {
    _data_type: std::marker::PhantomData<T>,
    #[allow(unused)]
    queue_name: std::ffi::CString,
    descriptor: libc::mqd_t,
}

impl<T: Sendable> MessageQueue<T> {
    #[allow(unused)]
    pub fn try_new<ID: AsRef<str>>(queue_name: ID, flags: MQueueFlags) -> Option<Self> {
        let name = std::ffi::CString::new(queue_name.as_ref()).ok()?;
        let user_rw_perms = 0o600;

        let open_rv = unsafe {
            libc::mq_open(
                name.as_c_str().as_ptr(),
                flags.as_c_int(),
                user_rw_perms,
                std::ptr::null::<libc::mq_attr>(),
            )
        };

        match open_rv {
            -1 => None,
            descriptor => Some(Self {
                _data_type: std::marker::PhantomData,
                queue_name: name,
                descriptor,
            }),
        }
    }

    #[allow(unused)]
    pub fn try_new_with_attr<ID: AsRef<str>>(
        queue_name: ID,
        flags: MQueueFlags,
        attr: MQueueAttr,
    ) -> Option<Self> {
        let name = std::ffi::CString::new(queue_name.as_ref()).ok()?;
        let attr_ptr = attr.to_mq_attr();
        let user_rw_perms = 0o600;

        let open_rv = unsafe {
            libc::mq_open(
                name.as_c_str().as_ptr(),
                flags.as_c_int(),
                user_rw_perms,
                &attr_ptr,
            )
        };

        match open_rv {
            -1 => None,
            descriptor => Some(Self {
                _data_type: std::marker::PhantomData,
                queue_name: name,
                descriptor,
            }),
        }
    }
}

impl<T> std::io::Write for MessageQueue<T>
where
    T: Sendable,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // lying to c by casting the pointer.
        let cast_ptr = buf.as_ptr() as *const i8;
        let data_size = buf.len();

        let rv = unsafe { libc::mq_send(self.descriptor, cast_ptr, buf.len(), 0) };
        match rv {
            0 => Ok(data_size),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "data exceeds message size",
            )),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<T> std::io::Read for MessageQueue<T>
where
    T: Sendable,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // lying to c by casting the pointer.
        let cast_ptr = buf.as_mut_ptr() as *mut i8;

        let read_bytes = unsafe {
            libc::mq_receive(self.descriptor, cast_ptr, 8192, std::ptr::null_mut::<u32>())
        };

        match read_bytes {
            -1 => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "unable to read from queue",
            )),
            read => usize::try_from(read).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "read bytes falls outside expected range.",
                )
            }),
        }
    }
}

impl<T: Sendable> Drop for MessageQueue<T> {
    fn drop(&mut self) {
        unsafe { libc::mq_close(self.descriptor) };
    }
}

struct SharedMem<T: Sendable> {
    _data_type: std::marker::PhantomData<T>,
    _shm_name: std::ffi::CString,
}

impl<T: Sendable> SharedMem<T> {
    fn try_new<ID: AsRef<str>>(shm_name: ID) -> Option<Self> {
        let name = std::ffi::CString::new(shm_name.as_ref()).ok()?;
        Some(Self {
            _data_type: std::marker::PhantomData,
            _shm_name: name,
        })
    }
}

pub fn bounded_sync_sender<ID, T>(queue_id: ID) -> Option<Sender<T>>
where
    ID: AsRef<str>,
    T: Sendable,
{
    let name = queue_id.as_ref();

    let mq_flags = MQueueFlags::new(MQueueMode::WriteOnly).with_option(MQueueOption::Create);
    let mq = MessageQueue::<T>::try_new(name, mq_flags)
        .map(RefCell::new)
        .map(Arc::new)?;

    let shm = SharedMem::<T>::try_new(name).map(Arc::new)?;
    let _sender = Sender::new(shm, mq);

    None
}
