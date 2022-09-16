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

pub struct Sender<'shmfd, T: Sendable, const CAP: usize> {
    _mmap: Arc<RefCell<MMap<'shmfd, T, CAP>>>,
    mqueue: Arc<RefCell<MessageQueue<T>>>,
}

impl<'shmfd, T: Sendable, const CAP: usize> Sender<'shmfd, T, CAP> {
    pub fn new(
        _mmap: Arc<RefCell<MMap<'shmfd, T, CAP>>>,
        mqueue: Arc<RefCell<MessageQueue<T>>>,
    ) -> Self {
        Self { _mmap, mqueue }
    }
}

impl<'shmfd, T, const CAP: usize> std::io::Write for Sender<'shmfd, T, CAP>
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

pub struct Receiver<T: Sendable, const CAP: usize> {
    _shm: Arc<SharedMem<T, CAP>>,
    mqueue: Arc<RefCell<MessageQueue<T>>>,
}

impl<T, const CAP: usize> std::io::Read for Receiver<T, CAP>
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
    fn data_size() -> usize {
        8192
    }

    fn encode(&self) -> &[u8];
}

impl Sendable for std::ffi::CStr {
    fn encode(&self) -> &[u8] {
        self.to_bytes_with_nul()
    }
}

impl Sendable for std::ffi::CString {
    fn encode(&self) -> &[u8] {
        self.as_c_str().encode()
    }
}

impl Sendable for &[u8] {
    fn encode(&self) -> &[u8] {
        self
    }
}

impl Sendable for &str {
    fn encode(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Sendable for String {
    fn encode(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<const N: usize> Sendable for [u8; N] {
    fn data_size() -> usize {
        N
    }

    fn encode(&self) -> &[u8] {
        self.as_slice()
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
        let name_ptr = name.as_ptr();

        let open_rv = unsafe {
            libc::mq_open(
                name_ptr,
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

    pub fn try_new_with_attr<ID: AsRef<str>>(
        queue_name: ID,
        flags: MQueueFlags,
        attr: MQueueAttr,
    ) -> Option<Self> {
        let queue_str = queue_name.as_ref();

        let name = std::ffi::CString::new(queue_str).ok()?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum MMapProt {
    None = libc::PROT_NONE,
    Read = libc::PROT_READ,
    Write = libc::PROT_WRITE,
    Exec = libc::PROT_EXEC,
}

impl MMapProt {
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    pub fn as_c_int(&self) -> libc::c_int {
        *self as libc::c_int
    }
}

#[repr(i32)]
pub enum MMapMode {
    Shared = libc::MAP_SHARED,
    SharedValidate = libc::MAP_SHARED_VALIDATE,
    Private = libc::MAP_PRIVATE,
}

#[repr(i32)]
pub enum MMapOption {
    M32Bit = libc::MAP_32BIT,
    Anonymous = libc::MAP_ANONYMOUS,
    DenyWrite = libc::MAP_DENYWRITE,
    Executable = libc::MAP_EXECUTABLE,
    File = libc::MAP_FILE,
    Fixed = libc::MAP_FIXED,
    FixedNoReplace = libc::MAP_FIXED_NOREPLACE,
    GrowsDown = libc::MAP_GROWSDOWN,
    HugeTLB = libc::MAP_HUGETLB,
    HugeTLB2MB = libc::MAP_HUGE_2MB,
    HugeTLB1GB = libc::MAP_HUGE_1GB,
    Locked = libc::MAP_LOCKED,
    NonBlock = libc::MAP_NONBLOCK,
    NoReserve = libc::MAP_NORESERVE,
    Populate = libc::MAP_POPULATE,
    Stack = libc::MAP_STACK,
    Sync = libc::MAP_SYNC,
}

pub struct MMapFlags(libc::c_int);

impl MMapFlags {
    pub fn new(intial_mode: MMapMode) -> Self {
        Self(intial_mode as i32)
    }

    pub fn with_option(self, option: MMapOption) -> Self {
        let new_flags = self.0 | (option as libc::c_int);
        Self(new_flags)
    }
}

impl MMapFlags {
    pub fn as_i32(&self) -> i32 {
        self.0
    }

    pub fn as_c_int(&self) -> libc::c_int {
        self.0
    }
}

pub struct MMap<'fd, T: Sendable, const CAP: usize> {
    _data_type: std::marker::PhantomData<T>,
    raw_ptr: *mut libc::c_void,
    len: usize,
    _fd: &'fd SharedMemDescriptor,
}

impl<'fd, T: Sendable, const CAP: usize> MMap<'fd, T, CAP> {
    pub fn try_new(fd: &'fd SharedMemDescriptor, prot: MMapProt, flags: MMapFlags) -> Option<Self> {
        let len = CAP;
        let fd_ptr = fd.as_ref();
        let raw_ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                prot.as_c_int(),
                flags.as_c_int(),
                *fd_ptr,
                0,
            )
        };

        (!raw_ptr.is_null()).then_some(Self {
            _data_type: std::marker::PhantomData,
            raw_ptr,
            len,
            _fd: fd,
        })
    }
}

impl<'fd, T: Sendable, const CAP: usize> AsRef<[T]> for MMap<'fd, T, CAP> {
    fn as_ref(&self) -> &[T] {
        let slice = unsafe {
            let ptr: &T = &*(self.raw_ptr as *const T);
            std::slice::from_raw_parts(ptr, CAP)
        };

        slice
    }
}

impl<'fd, T: Sendable, const CAP: usize> AsMut<[T]> for MMap<'fd, T, CAP> {
    fn as_mut(&mut self) -> &mut [T] {
        let slice = unsafe {
            let ptr: &mut T = &mut *(self.raw_ptr as *mut T);
            std::slice::from_raw_parts_mut(ptr, CAP)
        };

        slice
    }
}

impl<'fd, T: Sendable, const CAP: usize> Drop for MMap<'fd, T, CAP> {
    fn drop(&mut self) {
        if !self.raw_ptr.is_null() {
            unsafe { libc::munmap(self.raw_ptr, self.len) };
        }
    }
}

pub struct SharedMemDescriptor(libc::c_int);

impl SharedMemDescriptor {
    pub fn try_new(descriptor: libc::c_int) -> Option<Self> {
        (descriptor != 0).then_some(Self(descriptor))
    }
}

impl AsRef<libc::c_int> for SharedMemDescriptor {
    fn as_ref(&self) -> &libc::c_int {
        &self.0
    }
}

#[repr(i32)]
pub enum SharedMemMode {
    ReadOnly = libc::O_RDONLY,
    ReadWrite = libc::O_RDWR,
}

#[repr(i32)]
pub enum SharedMemOption {
    Create = libc::O_CREAT,
    Exclusive = libc::O_EXCL,
    Truncate = libc::O_TRUNC,
}

#[derive(Default)]
pub struct SharedMemFlags(libc::c_int);

impl SharedMemFlags {
    pub fn new(mode: SharedMemMode) -> Self {
        Self(mode as i32)
    }

    pub fn with_option(self, option: SharedMemOption) -> Self {
        let new_flags = self.0 | (option as i32);
        Self(new_flags)
    }
}

impl SharedMemFlags {
    pub fn as_i32(&self) -> i32 {
        self.0
    }

    pub fn as_c_int(&self) -> libc::c_int {
        self.0
    }
}

pub struct SharedMem<T: Sendable, const CAP: usize> {
    _data_type: std::marker::PhantomData<T>,
    shm_name: std::ffi::CString,
    descriptor: SharedMemDescriptor,
}

impl<T: Sendable, const CAP: usize> SharedMem<T, CAP> {
    #[allow(unused)]
    pub fn try_new<ID: AsRef<str>>(shm_name: ID, flags: SharedMemFlags) -> Option<Self> {
        let name = std::ffi::CString::new(shm_name.as_ref()).ok()?;
        let user_rw_perms = 0o600;
        let buf_size = std::mem::size_of::<T>() * CAP;

        let off_t_buf_size = libc::off_t::try_from(buf_size).ok()?;

        let open_rv =
            unsafe { libc::shm_open(name.as_c_str().as_ptr(), flags.as_c_int(), user_rw_perms) };

        // truncate if opened, successfully, returning the file descriptor.
        let fd = match open_rv {
            -1 => None,
            descriptor => Some(descriptor),
        }
        .and_then(|fd| match unsafe { libc::ftruncate(fd, off_t_buf_size) } {
            0 => Some(fd),
            _ => None,
        })
        .and_then(SharedMemDescriptor::try_new)?;

        Some(Self {
            _data_type: std::marker::PhantomData,
            shm_name: name,
            descriptor: fd,
        })
    }

    pub fn borrow_descriptor(&self) -> &SharedMemDescriptor {
        &self.descriptor
    }
}

impl<T: Sendable, const CAP: usize> Drop for SharedMem<T, CAP> {
    fn drop(&mut self) {
        let descriptor = self.descriptor.as_ref();
        unsafe { libc::close(*descriptor) };
    }
}

pub fn bounded_sync_sender<T, const CAP: usize>(shm: &SharedMem<T, CAP>) -> Option<Sender<T, CAP>>
where
    T: Sendable,
{
    let shm_name = shm.shm_name.to_str().ok()?;
    let queue_name = format!("/{}", shm_name);

    let attr_msgsize = i64::try_from(T::data_size()).ok()?;

    let mq = MessageQueue::<T>::try_new_with_attr(
        &queue_name,
        MQueueFlags::new(MQueueMode::WriteOnly).with_option(MQueueOption::Create),
        MQueueAttr::new(0, 10, attr_msgsize, 0),
    )?;

    let mmap_flags = MMapFlags::new(MMapMode::Shared);
    let mmap: Arc<RefCell<MMap<T, CAP>>> =
        MMap::try_new(&shm.descriptor, MMapProt::Write, mmap_flags)
            .map(RefCell::new)
            .map(Arc::new)?;

    Some(Sender::new(mmap, Arc::new(RefCell::new(mq))))
}
