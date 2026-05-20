use std::os::unix::io::RawFd;

#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub fd: RawFd,
    pub readable: bool,
    pub writable: bool,
    pub error: bool,
}

pub(crate) trait SelectorBackend: std::fmt::Debug {
    fn register(&mut self, fd: RawFd, readable: bool, writable: bool) -> Result<(), String>;
    fn unregister(&mut self, fd: RawFd) -> Result<(), String>;
    fn modify(&mut self, fd: RawFd, readable: bool, writable: bool) -> Result<(), String>;
    fn wait(&mut self, timeout_ms: i32) -> Result<Vec<Event>, String>;
    fn is_empty(&self) -> bool;
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::os::unix::io::RawFd;

    #[repr(C, packed)]
    #[derive(Debug, Clone, Copy)]
    struct EpollEvent {
        events: u32,
        data: u64,
    }

    const EPOLL_CTL_ADD: i32 = 1;
    const EPOLL_CTL_DEL: i32 = 2;
    const EPOLL_CTL_MOD: i32 = 3;
    const EPOLLIN: u32 = 0x001;
    const EPOLLOUT: u32 = 0x004;
    const EPOLLERR: u32 = 0x008;
    const EPOLLHUP: u32 = 0x010;
    const EPOLL_CLOEXEC: i32 = 0o2000000;
    const EPOLL_MAX_EVENTS: usize = 256;

    unsafe extern "C" {
        fn epoll_create1(flags: i32) -> i32;
        fn epoll_ctl(epfd: i32, op: i32, fd: i32, event: *const EpollEvent) -> i32;
        fn epoll_wait(epfd: i32, events: *mut EpollEvent, maxevents: i32, timeout: i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    #[derive(Debug)]
    pub struct Selector {
        epfd: RawFd,
        registered: Vec<RawFd>,
    }

    impl Selector {
        pub fn new() -> Result<Self, String> {
            let epfd = unsafe { epoll_create1(EPOLL_CLOEXEC) };
            if epfd < 0 {
                return Err(format!("epoll_create1 failed: {epfd}"));
            }
            Ok(Selector {
                epfd,
                registered: Vec::new(),
            })
        }
    }

    impl SelectorBackend for Selector {
        fn register(&mut self, fd: RawFd, readable: bool, writable: bool) -> Result<(), String> {
            let mut events = 0u32;
            if readable { events |= EPOLLIN; }
            if writable { events |= EPOLLOUT; }
            events |= EPOLLERR | EPOLLHUP;

            let ev = EpollEvent { events, data: fd as u64 };
            let ret = unsafe { epoll_ctl(self.epfd, EPOLL_CTL_ADD, fd, &ev) };
            if ret < 0 {
                return Err(format!("epoll_ctl ADD failed for fd {fd}: {ret}"));
            }
            self.registered.push(fd);
            Ok(())
        }

        fn unregister(&mut self, fd: RawFd) -> Result<(), String> {
            let ev = EpollEvent { events: 0, data: 0 };
            let ret = unsafe { epoll_ctl(self.epfd, EPOLL_CTL_DEL, fd, &ev) };
            if ret < 0 {
                return Err(format!("epoll_ctl DEL failed for fd {fd}: {ret}"));
            }
            self.registered.retain(|&f| f != fd);
            Ok(())
        }

        fn modify(&mut self, fd: RawFd, readable: bool, writable: bool) -> Result<(), String> {
            let mut events = 0u32;
            if readable { events |= EPOLLIN; }
            if writable { events |= EPOLLOUT; }
            let ev = EpollEvent { events, data: fd as u64 };
            let ret = unsafe { epoll_ctl(self.epfd, EPOLL_CTL_MOD, fd, &ev) };
            if ret < 0 {
                return Err(format!("epoll_ctl MOD failed for fd {fd}: {ret}"));
            }
            Ok(())
        }

        fn wait(&mut self, timeout_ms: i32) -> Result<Vec<Event>, String> {
            let mut raw_events = vec![
                EpollEvent {
                    events: 0,
                    data: 0
                };
                EPOLL_MAX_EVENTS
            ];
            let n = unsafe {
                epoll_wait(
                    self.epfd,
                    raw_events.as_mut_ptr(),
                    EPOLL_MAX_EVENTS as i32,
                    timeout_ms,
                )
            };
            if n < 0 {
                return Err(format!("epoll_wait failed: {n}"));
            }
            let mut result = Vec::with_capacity(n as usize);
            for i in 0..n as usize {
                let ev = &raw_events[i];
                result.push(Event {
                    fd: ev.data as RawFd,
                    readable: (ev.events & EPOLLIN) != 0,
                    writable: (ev.events & EPOLLOUT) != 0,
                    error: (ev.events & (EPOLLERR | EPOLLHUP)) != 0,
                });
            }
            Ok(result)
        }

        fn is_empty(&self) -> bool {
            self.registered.is_empty()
        }
    }

    impl Drop for Selector {
        fn drop(&mut self) {
            unsafe {
                close(self.epfd);
            }
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
mod platform {
    use super::*;
    use std::os::unix::io::RawFd;

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    struct Kevent {
        ident: u64,
        filter: i16,
        flags: u16,
        fflags: u32,
        data: isize,
        udata: *mut std::ffi::c_void,
    }

    const EVFILT_READ: i16 = -1;
    const EVFILT_WRITE: i16 = -2;
    const EV_ADD: u16 = 0x0001;
    const EV_DELETE: u16 = 0x0002;
    const EV_ENABLE: u16 = 0x0004;
    const EV_ONESHOT: u16 = 0x0010;
    const EV_EOF: u16 = 0x8000;
    const EV_ERROR: u16 = 0x4000;
    const KQ_MAX_EVENTS: usize = 256;

    unsafe extern "C" {
        fn kqueue() -> i32;
        fn kevent(
            kq: i32,
            changelist: *const Kevent,
            nchanges: i32,
            eventlist: *mut Kevent,
            nevents: i32,
            timeout: *const std::ffi::c_void,
        ) -> i32;
        fn close(fd: i32) -> i32;
    }

    #[derive(Debug)]
    pub struct Selector {
        kq: RawFd,
        registered: Vec<RawFd>,
    }

    impl Selector {
        pub fn new() -> Result<Self, String> {
            let kq = unsafe { kqueue() };
            if kq < 0 {
                return Err(format!("kqueue() failed: {kq}"));
            }
            Ok(Selector {
                kq,
                registered: Vec::new(),
            })
        }
    }

    impl SelectorBackend for Selector {
        fn register(&mut self, fd: RawFd, readable: bool, writable: bool) -> Result<(), String> {
            let mut changes = Vec::new();
            if readable {
                changes.push(Kevent {
                    ident: fd as u64,
                    filter: EVFILT_READ,
                    flags: EV_ADD | EV_ENABLE,
                    fflags: 0,
                    data: 0,
                    udata: std::ptr::null_mut(),
                });
            }
            if writable {
                changes.push(Kevent {
                    ident: fd as u64,
                    filter: EVFILT_WRITE,
                    flags: EV_ADD | EV_ENABLE,
                    fflags: 0,
                    data: 0,
                    udata: std::ptr::null_mut(),
                });
            }
            let ret = unsafe {
                kevent(
                    self.kq,
                    changes.as_ptr(),
                    changes.len() as i32,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                )
            };
            if ret < 0 {
                return Err(format!("kevent register failed for fd {fd}: {ret}"));
            }
            self.registered.push(fd);
            Ok(())
        }

        fn unregister(&mut self, fd: RawFd) -> Result<(), String> {
            let changes = [
                Kevent {
                    ident: fd as u64,
                    filter: EVFILT_READ,
                    flags: EV_DELETE,
                    fflags: 0,
                    data: 0,
                    udata: std::ptr::null_mut(),
                },
                Kevent {
                    ident: fd as u64,
                    filter: EVFILT_WRITE,
                    flags: EV_DELETE,
                    fflags: 0,
                    data: 0,
                    udata: std::ptr::null_mut(),
                },
            ];
            let ret = unsafe {
                kevent(
                    self.kq,
                    changes.as_ptr(),
                    2,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                )
            };
            if ret < 0 {
                return Err(format!("kevent unregister failed for fd {fd}: {ret}"));
            }
            self.registered.retain(|&f| f != fd);
            Ok(())
        }

        fn modify(&mut self, fd: RawFd, readable: bool, writable: bool) -> Result<(), String> {
            let _ = self.unregister(fd);
            let _ = self.register(fd, readable, writable);
            Ok(())
        }

        fn wait(&mut self, timeout_ms: i32) -> Result<Vec<Event>, String> {
            let mut raw_events = vec![
                Kevent {
                    ident: 0,
                    filter: 0,
                    flags: 0,
                    fflags: 0,
                    data: 0,
                    udata: std::ptr::null_mut()
                };
                KQ_MAX_EVENTS
            ];

            let ts = if timeout_ms >= 0 {
                let sec = timeout_ms / 1000;
                let nsec = (timeout_ms % 1000) * 1_000_000;
                Some(libc::timespec {
                    tv_sec: sec as libc::time_t,
                    tv_nsec: nsec as libc::c_long,
                })
            } else {
                None
            };
            let ts_ptr = ts.as_ref().map_or(std::ptr::null(), |t| t as *const _ as *const std::ffi::c_void);

            let n = unsafe {
                kevent(
                    self.kq,
                    std::ptr::null(),
                    0,
                    raw_events.as_mut_ptr(),
                    KQ_MAX_EVENTS as i32,
                    ts_ptr,
                )
            };
            if n < 0 {
                return Err(format!("kevent wait failed: {n}"));
            }
            let mut result = Vec::with_capacity(n as usize);
            for i in 0..n as usize {
                let ev = &raw_events[i];
                let error = (ev.flags & (EV_EOF | EV_ERROR)) != 0;
                result.push(Event {
                    fd: ev.ident as RawFd,
                    readable: ev.filter == EVFILT_READ,
                    writable: ev.filter == EVFILT_WRITE,
                    error,
                });
            }
            Ok(result)
        }

        fn is_empty(&self) -> bool {
            self.registered.is_empty()
        }
    }

    impl Drop for Selector {
        fn drop(&mut self) {
            unsafe {
                close(self.kq);
            }
        }
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
mod platform {
    use super::*;
    use std::os::unix::io::RawFd;

    #[derive(Debug)]
    pub struct Selector {
        fds: Vec<RawFd>,
        max_fd: RawFd,
    }

    impl Selector {
        pub fn new() -> Result<Self, String> {
            Ok(Selector {
                fds: Vec::new(),
                max_fd: 0,
            })
        }
    }

    impl SelectorBackend for Selector {
        fn register(&mut self, fd: RawFd, _readable: bool, _writable: bool) -> Result<(), String> {
            self.fds.push(fd);
            self.max_fd = self.max_fd.max(fd);
            Ok(())
        }

        fn unregister(&mut self, fd: RawFd) -> Result<(), String> {
            self.fds.retain(|&f| f != fd);
            self.max_fd = self.fds.iter().copied().max().unwrap_or(0);
            Ok(())
        }

        fn modify(&mut self, _fd: RawFd, _readable: bool, _writable: bool) -> Result<(), String> {
            Ok(())
        }

        fn wait(&mut self, timeout_ms: i32) -> Result<Vec<Event>, String> {
            use std::mem;
            use std::ptr;

            let nfds = (self.max_fd + 1) as i32;
            let byte_size = ((nfds + 7) / 8) as usize;

            let mut read_set = vec![0u8; byte_size];
            let mut write_set = vec![0u8; byte_size];

            for &fd in &self.fds {
                let pos = fd as usize / 8;
                let bit = 1u8 << (fd as usize % 8);
                read_set[pos] |= bit;
                write_set[pos] |= bit;
            }

            let ts = if timeout_ms >= 0 {
                let sec = timeout_ms / 1000;
                let usec = (timeout_ms % 1000) * 1000;
                Some(libc::timeval {
                    tv_sec: sec as libc::time_t,
                    tv_usec: usec as libc::suseconds_t,
                })
            } else {
                None
            };

            let ret = unsafe {
                libc::select(
                    nfds,
                    &mut read_set as *mut _ as *mut libc::fd_set,
                    &mut write_set as *mut _ as *mut libc::fd_set,
                    ptr::null_mut(),
                    ts.as_ref().map_or(ptr::null(), |t| t as *const _),
                )
            };
            if ret < 0 {
                return Err(format!("select failed: {ret}"));
            }

            let mut result = Vec::new();
            for &fd in &self.fds {
                let pos = fd as usize / 8;
                let bit = 1u8 << (fd as usize % 8);
                let readable = (read_set[pos] & bit) != 0;
                let writable = (write_set[pos] & bit) != 0;
                if readable || writable {
                    result.push(Event {
                        fd,
                        readable,
                        writable,
                        error: false,
                    });
                }
            }
            Ok(result)
        }

        fn is_empty(&self) -> bool {
            self.fds.is_empty()
        }
    }
}

#[derive(Debug)]
pub struct Poller {
    inner: platform::Selector,
}

impl Poller {
    pub fn new() -> Result<Self, String> {
        platform::Selector::new().map(|inner| Poller { inner })
    }

    pub fn register(&mut self, fd: RawFd, readable: bool, writable: bool) -> Result<(), String> {
        self.inner.register(fd, readable, writable)
    }

    pub fn unregister(&mut self, fd: RawFd) -> Result<(), String> {
        self.inner.unregister(fd)
    }

    pub fn modify(&mut self, fd: RawFd, readable: bool, writable: bool) -> Result<(), String> {
        self.inner.modify(fd, readable, writable)
    }

    pub fn wait(&mut self, timeout_ms: i32) -> Result<Vec<Event>, String> {
        self.inner.wait(timeout_ms)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
