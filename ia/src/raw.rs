pub use executor::spawn;

use std::boxed::FnBox;
use std::rc::Rc;
use std::ops::Deref;
use std::io::SeekFrom;

use error;
use error::IoResult;

extern "C" {
    fn __ice_tcp_connect(
        addr_base: *const u8,
        addr_len: usize,
        cb: extern "C" fn (user_data: i32, stream_tid: i32) -> i32,
        user_data: i32
    );
    fn __ice_tcp_listen(
        addr_base: *const u8,
        addr_len: usize,
        cb: extern "C" fn (user_data: i32, stream_tid: i32) -> i32,
        user_data: i32
    ) -> i32;
    fn __ice_tcp_release_buffer(
        buffer_id: i32
    );
    fn __ice_tcp_take_buffer(
        buffer_id: i32,
        output: *mut u8,
        output_len: usize
    ) -> usize;
    fn __ice_tcp_read(
        stream_tid: i32,
        read_len: usize,
        cb: extern "C" fn (user_data: i32, len: i32) -> i32,
        user_data: i32
    );
    fn __ice_tcp_write(
        stream_tid: i32,
        data_base: *const u8,
        data_len: usize,
        cb: extern "C" fn (user_data: i32, len: i32) -> i32,
        user_data: i32
    );
    fn __ice_tcp_destroy(stream_tid: i32);
    fn __ice_file_open(
        path_base: *const u8,
        path_len: usize,
        mode_base: *const u8,
        mode_len: usize
    ) -> i32;
    fn __ice_file_close(fd: i32);
    fn __ice_file_read(
        fd: i32,
        buf_base: *mut u8,
        buf_len: usize
    ) -> i32;
    fn __ice_file_write(
        fd: i32,
        buf_base: *const u8,
        buf_len: usize
    ) -> i32;
    fn __ice_file_flush(fd: i32) -> i32;
    fn __ice_file_seek(
        fd: i32,
        from: i32,
        offset: i64
    ) -> i64;
    fn __ice_timer_now_millis() -> i64;
    fn __ice_timer_set_immediate(cb: extern "C" fn (user_data: i32) -> i32, user_data: i32);
    fn __ice_logging_info(base: *const u8, len: usize);
    fn __ice_logging_warning(base: *const u8, len: usize);
}

#[macro_export]
macro_rules! app_init {
    ($body:block) => {
        #[no_mangle]
        pub extern "C" fn __app_init() -> i32 {
            $body
        }
    }
}

#[no_mangle]
pub extern "C" fn __app_invoke0(
    target: extern "C" fn () -> i32
) -> i32 {
    target()
}

#[no_mangle]
pub extern "C" fn __app_invoke1(
    target: extern "C" fn (i32) -> i32,
    arg1: i32
) -> i32 {
    target(arg1)
}

#[no_mangle]
pub extern "C" fn __app_invoke2(
    target: extern "C" fn (i32, i32) -> i32,
    arg1: i32,
    arg2: i32
) -> i32 {
    target(arg1, arg2)
}

#[no_mangle]
pub extern "C" fn __app_invoke3(
    target: extern "C" fn (i32, i32, i32) -> i32,
    arg1: i32,
    arg2: i32,
    arg3: i32
) -> i32 {
    target(arg1, arg2, arg3)
}

#[no_mangle]
pub extern "C" fn __app_invoke4(
    target: extern "C" fn (i32, i32, i32, i32) -> i32,
    arg1: i32,
    arg2: i32,
    arg3: i32,
    arg4: i32
) -> i32 {
    target(arg1, arg2, arg3, arg4)
}

pub trait WrapCallback {
    type Function;

    fn wrap_callback(self) -> (Self::Function, i32);
}

macro_rules! impl_wrap_callback {
    ($($arg_name:ident : $arg_t:ty, )*) => {
        impl WrapCallback for Box<Fn($($arg_t, )*) -> i32> {
            type Function = extern "C" fn (i32 $(, $arg_name: $arg_t)*) -> i32;

            fn wrap_callback(self) -> (Self::Function, i32) {
                extern "C" fn raw_cb(addr: i32 $(, $arg_name: $arg_t)*) -> i32 {
                    let f: &Box<Fn($($arg_t, )*) -> i32> = unsafe {
                        &* (addr as *const Box<Fn($($arg_t, )*) -> i32>)
                    };
                    f($($arg_name, )*)
                }
                let f: Box<Box<Fn($($arg_t, )*) -> i32>> = Box::new(self);
                let f = Box::into_raw(f);
                (raw_cb, f as _)
            }
        }
        impl WrapCallback for Box<FnBox($($arg_t, )*) -> i32> {
            type Function = extern "C" fn (i32 $(, $arg_name: $arg_t)*) -> i32;

            fn wrap_callback(self) -> (Self::Function, i32) {
                extern "C" fn raw_cb(addr: i32 $(, $arg_name: $arg_t)*) -> i32 {
                    let f: Box<Box<FnBox($($arg_t, )*) -> i32>> = unsafe {
                        Box::from_raw(addr as *mut Box<FnBox($($arg_t, )*) -> i32>)
                    };
                    f($($arg_name, )*)
                }
                let f: Box<Box<FnBox($($arg_t, )*) -> i32>> = Box::new(self);
                let f = Box::into_raw(f);
                (raw_cb, f as _)
            }
        }
    }
}

impl_wrap_callback!();
impl_wrap_callback!(a: i32, );
impl_wrap_callback!(a: i32, b: i32, );
impl_wrap_callback!(a: i32, b: i32, c: i32, );
impl_wrap_callback!(a: i32, b: i32, c: i32, d: i32, );
impl_wrap_callback!(a: i32, b: i32, c: i32, d: i32, e: i32, );
impl_wrap_callback!(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, );

/*
pub fn set_timeout<T: FnOnce()>(ms: i64, cb: T) {
    unimplemented!()
}*/

pub fn schedule<T: FnOnce() + 'static>(cb: T) {
    let cb: Box<FnBox() -> i32> = Box::new(|| { cb(); 0 });
    let (cb, raw_ctx) = cb.wrap_callback();
    unsafe {
        __ice_timer_set_immediate(cb, raw_ctx);
    }
}

pub fn time() -> i64 {
    unsafe {
        __ice_timer_now_millis()
    }
}

#[derive(Clone)]
pub struct TcpStream {
    inner: Rc<TcpStreamImpl>
}

unsafe impl Send for TcpStream {}
unsafe impl Sync for TcpStream {}

impl Deref for TcpStream {
    type Target = TcpStreamImpl;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

pub struct TcpStreamImpl {
    handle: i32
}

impl Drop for TcpStreamImpl {
    fn drop(&mut self) {
        unsafe {
            __ice_tcp_destroy(self.handle);
        }
    }
}

pub struct TcpBuffer {
    handle: i32
}

impl Drop for TcpBuffer {
    fn drop(&mut self) {
        unsafe {
            __ice_tcp_release_buffer(self.handle);
        }
    }
}

impl TcpBuffer {
    pub fn take(self, out: &mut [u8]) -> usize {
        let out_len = out.len();
        let real_len = unsafe { __ice_tcp_take_buffer(
            self.handle,
            &mut out[0],
            out_len
        ) };
        ::std::mem::forget(self);
        real_len
    }
}

impl TcpStreamImpl {
    pub fn write<F: FnOnce(IoResult<i32>) + 'static>(&self, data: &[u8], cb: F) {
        if data.len() == 0 {
            cb(Err(error::Io::Generic));
            return;
        }

        let cb: Box<FnBox(i32) -> i32> = Box::new(|a| {
            cb(if a >= 0 {
                Ok(a)
            } else {
                Err(error::Io::Generic)
            });
            0
        });
        let (cb, raw_ctx) = cb.wrap_callback();

        unsafe {
            __ice_tcp_write(
                self.handle,
                &data[0],
                data.len(),
                cb,
                raw_ctx
            )
        }
    }

    pub fn read<F: FnOnce(IoResult<TcpBuffer>) + 'static>(&self, len: usize, cb: F) {
        let cb: Box<FnBox(i32) -> i32> = Box::new(|a| {
            cb(if a >= 0 {
                Ok(TcpBuffer { handle: a })
            } else {
                Err(error::Io::Generic)
            });
            0
        });
        let (cb, raw_ctx) = cb.wrap_callback();

        unsafe {
            __ice_tcp_read(
                self.handle,
                len,
                cb,
                raw_ctx
            )
        }
    }
}

pub fn listen_tcp<T: Fn(TcpStream) + 'static>(
    addr: &str,
    cb: T
) -> i32 {
    let cb: Box<Fn(i32) -> i32> = Box::new(move |stream_tid| {
        if stream_tid >= 0 {
            cb(TcpStream {
                inner: Rc::new(TcpStreamImpl {
                    handle: stream_tid
                })
            });
        }
        0
    });
    let (cb, raw_ctx) = cb.wrap_callback();

    unsafe {
        let addr = addr.as_bytes();
        __ice_tcp_listen(
            &addr[0],
            addr.len(),
            cb,
            raw_ctx
        )
    }
}

pub fn connect_tcp<F: FnOnce(IoResult<TcpStream>) + 'static>(
    addr: &str,
    cb: F
) {
    let cb: Box<FnBox(i32) -> i32> = Box::new(move |stream_tid| {
        cb(if stream_tid >= 0 {
            Ok(TcpStream {
                inner: Rc::new(TcpStreamImpl {
                    handle: stream_tid
                })
            })
        } else {
            Err(error::Io::Generic)
        });

        0
    });
    let (cb, raw_ctx) = cb.wrap_callback();

    unsafe {
        let addr = addr.as_bytes();
        __ice_tcp_connect(
            &addr[0],
            addr.len(),
            cb,
            raw_ctx
        )
    }
}

pub fn file_open(path: &str, mode: &str) -> IoResult<i32> {
    let path = path.as_bytes();
    let mode = mode.as_bytes();

    let ret = unsafe {
        __ice_file_open(
            &path[0],
            path.len(),
            &mode[0],
            mode.len()
        )
    };
    if ret >= 0 {
        Ok(ret)
    } else {
        Err(error::Io::Generic)
    }
}

pub fn file_close(fd: i32) {
    unsafe {
        __ice_file_close(fd);
    }
}

pub fn file_read(fd: i32, out: &mut [u8]) -> IoResult<usize> {
    let out_len = out.len();

    let ret = unsafe {
        __ice_file_read(
            fd,
            &mut out[0],
            out_len
        )
    };
    if ret >= 0 {
        Ok(ret as usize)
    } else {
        Err(error::Io::Generic)
    }
}

pub fn file_write(fd: i32, data: &[u8]) -> IoResult<usize> {
    let data_len = data.len();

    let ret = unsafe {
        __ice_file_write(
            fd,
            &data[0],
            data_len
        )
    };
    if ret >= 0 {
        Ok(ret as usize)
    } else {
        Err(error::Io::Generic)
    }
}

pub fn file_flush(fd: i32) -> IoResult<()> {
    let ret = unsafe {
        __ice_file_flush(fd)
    };
    if ret >= 0 {
        Ok(())
    } else {
        Err(error::Io::Generic)
    }
}

pub fn file_seek(fd: i32, from: SeekFrom) -> IoResult<u64> {
    let (from, offset) = match from {
        SeekFrom::Start(offset) => (0, offset as i64),
        SeekFrom::End(offset) => (1, offset),
        SeekFrom::Current(offset) => (2, offset)
    };

    let ret: i64 = unsafe {
        __ice_file_seek(
            fd,
            from,
            offset
        )
    };
    if ret >= 0 {
        Ok(ret as u64)
    } else {
        Err(error::Io::Generic)
    }
}
