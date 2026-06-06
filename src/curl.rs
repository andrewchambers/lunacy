//! Thin libcurl easy wrappers.
//!
//! This module is available with the `curl` feature and requires system
//! libcurl headers and library at build time.

use alloc::boxed::Box;
use core::{ffi::CStr, fmt, marker::PhantomData, ptr::NonNull, slice, time::Duration};

/// A crate-local result type for libcurl operations.
pub type Result<T> = core::result::Result<T, Error>;

/// A raw libcurl `CURLcode`.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Code(crate::ffi::c_int);

impl Code {
    /// The libcurl success code, `CURLE_OK`.
    pub const OK: Self = Self(0);

    /// Wraps a raw `CURLcode`.
    pub const fn raw(raw: crate::ffi::c_int) -> Self {
        Self(raw)
    }

    /// Returns the raw `CURLcode`.
    pub const fn as_raw(self) -> crate::ffi::c_int {
        self.0
    }

    /// Returns whether this code is `CURLE_OK`.
    pub const fn is_ok(self) -> bool {
        self.0 == Self::OK.0
    }

    /// Returns libcurl's static description for this code.
    pub fn description(self) -> &'static CStr {
        unsafe { CStr::from_ptr(lunacy_curl_easy_strerror(self.0)) }
    }
}

impl fmt::Debug for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Code").field(&self.0).finish()
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "curl code {}", self.0)
    }
}

/// A libcurl error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Error {
    code: Code,
}

impl Error {
    /// Creates an error from a raw libcurl code.
    pub const fn new(code: Code) -> Self {
        Self { code }
    }

    /// Returns the libcurl code.
    pub const fn code(self) -> Code {
        self.code
    }

    /// Returns libcurl's static description for this error.
    pub fn description(self) -> &'static CStr {
        self.code.description()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.code.fmt(f)
    }
}

/// Initializes libcurl global state.
///
/// # Safety
///
/// libcurl global initialization affects process-global state. Call this before
/// using libcurl from other threads, and do not race it with other libcurl
/// global initialization or cleanup calls.
pub unsafe fn global_init() -> Result<()> {
    cvt(unsafe { lunacy_curl_global_init() })
}

/// Cleans up libcurl global state.
///
/// # Safety
///
/// No libcurl handles may be in use, and no other thread may call libcurl while
/// cleanup runs.
pub unsafe fn global_cleanup() {
    unsafe {
        lunacy_curl_global_cleanup();
    }
}

/// A libcurl linked list of strings.
///
/// This is used for options such as `CURLOPT_HTTPHEADER`.
pub struct Slist {
    raw: *mut crate::ffi::c_void,
}

impl Slist {
    /// Creates an empty string list.
    pub const fn new() -> Self {
        Self {
            raw: core::ptr::null_mut(),
        }
    }

    /// Appends a string to this list.
    pub fn append(&mut self, value: &CStr) -> Result<()> {
        cvt(unsafe { lunacy_curl_slist_append(&mut self.raw, value.as_ptr()) })
    }

    /// Returns the raw `curl_slist` pointer.
    pub const fn as_raw(&self) -> *mut crate::ffi::c_void {
        self.raw
    }

    /// Returns whether this list is empty.
    pub const fn is_empty(&self) -> bool {
        self.raw.is_null()
    }
}

impl Default for Slist {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Slist {
    fn drop(&mut self) {
        unsafe {
            lunacy_curl_slist_free_all(self.raw);
        }
    }
}

/// Action returned by a write callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteAction {
    /// Continue the transfer.
    Continue,
    /// Abort the transfer.
    Abort,
}

/// Transfer progress reported by libcurl.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XferInfo {
    download_total: i64,
    download_now: i64,
    upload_total: i64,
    upload_now: i64,
}

impl XferInfo {
    /// Returns the expected download byte count, or zero if unknown.
    pub const fn download_total(self) -> i64 {
        self.download_total
    }

    /// Returns the downloaded byte count so far.
    pub const fn download_now(self) -> i64 {
        self.download_now
    }

    /// Returns the expected upload byte count, or zero if unknown.
    pub const fn upload_total(self) -> i64 {
        self.upload_total
    }

    /// Returns the uploaded byte count so far.
    pub const fn upload_now(self) -> i64 {
        self.upload_now
    }
}

/// Action returned by an xferinfo callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XferAction {
    /// Continue the transfer.
    Continue,
    /// Abort the transfer.
    Abort,
}

type WriteFn<'a> = dyn FnMut(&[u8]) -> WriteAction + 'a;
type XferFn<'a> = dyn FnMut(XferInfo) -> XferAction + 'a;

struct WriteCallback<'a> {
    callback: Box<WriteFn<'a>>,
}
struct XferCallback<'a> {
    callback: Box<XferFn<'a>>,
}

/// A libcurl easy handle.
pub struct Easy<'a> {
    raw: NonNull<crate::ffi::c_void>,
    write: Option<Box<WriteCallback<'a>>>,
    xfer: Option<Box<XferCallback<'a>>>,
    _marker: PhantomData<&'a mut ()>,
}

impl<'a> Easy<'a> {
    /// Creates a new easy handle.
    pub fn new() -> Result<Self> {
        let mut raw = core::ptr::null_mut();
        cvt(unsafe { lunacy_curl_easy_new(&mut raw) })?;
        let raw = NonNull::new(raw).ok_or_else(bad_argument)?;

        Ok(Self {
            raw,
            write: None,
            xfer: None,
            _marker: PhantomData,
        })
    }

    /// Sets `CURLOPT_URL`.
    pub fn set_url(&mut self, url: &CStr) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_url(self.raw.as_ptr(), url.as_ptr()) })
    }

    /// Sets `CURLOPT_FOLLOWLOCATION`.
    pub fn set_follow_location(&mut self, enabled: bool) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_follow_location(self.raw.as_ptr(), c_bool(enabled)) })
    }

    /// Sets `CURLOPT_VERBOSE`.
    pub fn set_verbose(&mut self, enabled: bool) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_verbose(self.raw.as_ptr(), c_bool(enabled)) })
    }

    /// Sets `CURLOPT_SSL_VERIFYPEER`.
    pub fn set_ssl_verifypeer(&mut self, enabled: bool) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_ssl_verifypeer(self.raw.as_ptr(), c_bool(enabled)) })
    }

    /// Sets `CURLOPT_SSL_VERIFYHOST`.
    pub fn set_ssl_verifyhost(&mut self, enabled: bool) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_ssl_verifyhost(self.raw.as_ptr(), c_bool(enabled)) })
    }

    /// Sets `CURLOPT_CAINFO`.
    pub fn set_ca_info(&mut self, path: &CStr) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_ca_info(self.raw.as_ptr(), path.as_ptr()) })
    }

    /// Clears `CURLOPT_CAINFO`.
    pub fn clear_ca_info(&mut self) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_ca_info(self.raw.as_ptr(), core::ptr::null()) })
    }

    /// Sets `CURLOPT_CAPATH`.
    pub fn set_ca_path(&mut self, path: &CStr) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_ca_path(self.raw.as_ptr(), path.as_ptr()) })
    }

    /// Clears `CURLOPT_CAPATH`.
    pub fn clear_ca_path(&mut self) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_ca_path(self.raw.as_ptr(), core::ptr::null()) })
    }

    /// Sets `CURLOPT_TIMEOUT_MS`.
    pub fn set_timeout(&mut self, timeout: Duration) -> Result<()> {
        cvt(unsafe {
            lunacy_curl_easy_set_timeout_ms(self.raw.as_ptr(), duration_millis(timeout)?)
        })
    }

    /// Sets `CURLOPT_CONNECTTIMEOUT_MS`.
    pub fn set_connect_timeout(&mut self, timeout: Duration) -> Result<()> {
        cvt(unsafe {
            lunacy_curl_easy_set_connect_timeout_ms(self.raw.as_ptr(), duration_millis(timeout)?)
        })
    }

    /// Sets `CURLOPT_POST`.
    pub fn set_post(&mut self, enabled: bool) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_post(self.raw.as_ptr(), c_bool(enabled)) })
    }

    /// Sets `CURLOPT_POSTFIELDSIZE_LARGE` and `CURLOPT_COPYPOSTFIELDS`.
    pub fn set_copy_post_fields(&mut self, body: &[u8]) -> Result<()> {
        cvt(unsafe {
            lunacy_curl_easy_set_copy_post_fields(
                self.raw.as_ptr(),
                body.as_ptr().cast(),
                body.len() as u64,
            )
        })
    }

    /// Sets `CURLOPT_HTTPHEADER`.
    ///
    /// # Safety
    ///
    /// `headers` must remain alive and unchanged until the option is cleared,
    /// replaced, or the transfer using it has completed.
    pub unsafe fn set_http_headers(&mut self, headers: &Slist) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_http_headers(self.raw.as_ptr(), headers.as_raw()) })
    }

    /// Clears `CURLOPT_HTTPHEADER`.
    pub fn clear_http_headers(&mut self) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_http_headers(self.raw.as_ptr(), core::ptr::null_mut()) })
    }

    /// Sets `CURLOPT_WRITEFUNCTION` and `CURLOPT_WRITEDATA`.
    pub fn set_write_function<F>(&mut self, callback: F) -> Result<()>
    where
        F: FnMut(&[u8]) -> WriteAction + 'a,
    {
        let mut callback = Box::new(WriteCallback {
            callback: Box::new(callback),
        });
        let userdata = (&mut *callback) as *mut WriteCallback<'a>;

        cvt(unsafe {
            lunacy_curl_easy_set_write_function(
                self.raw.as_ptr(),
                write_trampoline,
                userdata.cast(),
            )
        })?;
        self.write = Some(callback);
        Ok(())
    }

    /// Sets `CURLOPT_XFERINFOFUNCTION` and `CURLOPT_XFERINFODATA`.
    ///
    /// Call [`set_noprogress`](Self::set_noprogress) with `false` to make
    /// libcurl invoke this callback.
    pub fn set_xferinfo_function<F>(&mut self, callback: F) -> Result<()>
    where
        F: FnMut(XferInfo) -> XferAction + 'a,
    {
        let mut callback = Box::new(XferCallback {
            callback: Box::new(callback),
        });
        let userdata = (&mut *callback) as *mut XferCallback<'a>;

        cvt(unsafe {
            lunacy_curl_easy_set_xferinfo_function(
                self.raw.as_ptr(),
                xferinfo_trampoline,
                userdata.cast(),
            )
        })?;
        self.xfer = Some(callback);
        Ok(())
    }

    /// Sets `CURLOPT_NOPROGRESS`.
    pub fn set_noprogress(&mut self, enabled: bool) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_set_noprogress(self.raw.as_ptr(), c_bool(enabled)) })
    }

    /// Runs `curl_easy_perform`.
    pub fn perform(&mut self) -> Result<()> {
        cvt(unsafe { lunacy_curl_easy_perform(self.raw.as_ptr()) })
    }

    /// Returns `CURLINFO_RESPONSE_CODE`.
    pub fn response_code(&mut self) -> Result<u32> {
        let mut out = 0;
        cvt(unsafe { lunacy_curl_easy_get_response_code(self.raw.as_ptr(), &mut out) })?;
        Ok(out)
    }

    /// Returns this handle's `CURLOPT_ERRORBUFFER` contents.
    pub fn error_buffer(&self) -> &CStr {
        unsafe { CStr::from_ptr(lunacy_curl_easy_error_buffer(self.raw.as_ptr())) }
    }
}

impl Drop for Easy<'_> {
    fn drop(&mut self) {
        unsafe {
            lunacy_curl_easy_cleanup(self.raw.as_ptr());
        }
    }
}

extern "C" fn write_trampoline(
    userdata: *mut crate::ffi::c_void,
    data: *const u8,
    len: crate::ffi::size_t,
) -> crate::ffi::size_t {
    let callback = unsafe { &mut *userdata.cast::<WriteCallback<'_>>() };
    let data = unsafe { slice::from_raw_parts(data, len) };

    match (callback.callback)(data) {
        WriteAction::Continue => len,
        WriteAction::Abort => 0,
    }
}

extern "C" fn xferinfo_trampoline(
    userdata: *mut crate::ffi::c_void,
    download_total: i64,
    download_now: i64,
    upload_total: i64,
    upload_now: i64,
) -> crate::ffi::c_int {
    let callback = unsafe { &mut *userdata.cast::<XferCallback<'_>>() };
    let info = XferInfo {
        download_total,
        download_now,
        upload_total,
        upload_now,
    };

    match (callback.callback)(info) {
        XferAction::Continue => 0,
        XferAction::Abort => 1,
    }
}

fn cvt(code: crate::ffi::c_int) -> Result<()> {
    let code = Code(code);
    if code.is_ok() {
        Ok(())
    } else {
        Err(Error::new(code))
    }
}

fn bad_argument() -> Error {
    Error::new(Code(unsafe { lunacy_curl_bad_function_argument() }))
}

fn duration_millis(duration: Duration) -> Result<i64> {
    let millis = duration
        .as_secs()
        .checked_mul(1_000)
        .and_then(|millis| {
            millis.checked_add(u64::from(duration.subsec_nanos().div_ceil(1_000_000)))
        })
        .ok_or_else(bad_argument)?;

    if millis > i64::MAX as u64 {
        Err(bad_argument())
    } else {
        Ok(millis as i64)
    }
}

fn c_bool(value: bool) -> crate::ffi::c_int {
    if value { 1 } else { 0 }
}

type WriteTrampoline = extern "C" fn(
    userdata: *mut crate::ffi::c_void,
    data: *const u8,
    len: crate::ffi::size_t,
) -> crate::ffi::size_t;

type XferInfoTrampoline = extern "C" fn(
    userdata: *mut crate::ffi::c_void,
    download_total: i64,
    download_now: i64,
    upload_total: i64,
    upload_now: i64,
) -> crate::ffi::c_int;

unsafe extern "C" {
    fn lunacy_curl_global_init() -> crate::ffi::c_int;
    fn lunacy_curl_global_cleanup();
    fn lunacy_curl_easy_strerror(code: crate::ffi::c_int) -> *const crate::ffi::c_char;
    fn lunacy_curl_bad_function_argument() -> crate::ffi::c_int;
    fn lunacy_curl_easy_new(out: *mut *mut crate::ffi::c_void) -> crate::ffi::c_int;
    fn lunacy_curl_easy_cleanup(easy: *mut crate::ffi::c_void);
    fn lunacy_curl_easy_error_buffer(easy: *mut crate::ffi::c_void) -> *const crate::ffi::c_char;
    fn lunacy_curl_easy_perform(easy: *mut crate::ffi::c_void) -> crate::ffi::c_int;
    fn lunacy_curl_easy_get_response_code(
        easy: *mut crate::ffi::c_void,
        out: *mut u32,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_url(
        easy: *mut crate::ffi::c_void,
        url: *const crate::ffi::c_char,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_follow_location(
        easy: *mut crate::ffi::c_void,
        enabled: crate::ffi::c_int,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_verbose(
        easy: *mut crate::ffi::c_void,
        enabled: crate::ffi::c_int,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_ssl_verifypeer(
        easy: *mut crate::ffi::c_void,
        enabled: crate::ffi::c_int,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_ssl_verifyhost(
        easy: *mut crate::ffi::c_void,
        enabled: crate::ffi::c_int,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_ca_info(
        easy: *mut crate::ffi::c_void,
        path: *const crate::ffi::c_char,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_ca_path(
        easy: *mut crate::ffi::c_void,
        path: *const crate::ffi::c_char,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_timeout_ms(
        easy: *mut crate::ffi::c_void,
        millis: i64,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_connect_timeout_ms(
        easy: *mut crate::ffi::c_void,
        millis: i64,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_post(
        easy: *mut crate::ffi::c_void,
        enabled: crate::ffi::c_int,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_copy_post_fields(
        easy: *mut crate::ffi::c_void,
        data: *const crate::ffi::c_void,
        len: u64,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_http_headers(
        easy: *mut crate::ffi::c_void,
        list: *mut crate::ffi::c_void,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_write_function(
        easy: *mut crate::ffi::c_void,
        callback: WriteTrampoline,
        userdata: *mut crate::ffi::c_void,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_xferinfo_function(
        easy: *mut crate::ffi::c_void,
        callback: XferInfoTrampoline,
        userdata: *mut crate::ffi::c_void,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_easy_set_noprogress(
        easy: *mut crate::ffi::c_void,
        enabled: crate::ffi::c_int,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_slist_append(
        list: *mut *mut crate::ffi::c_void,
        value: *const crate::ffi::c_char,
    ) -> crate::ffi::c_int;
    fn lunacy_curl_slist_free_all(list: *mut crate::ffi::c_void);
}
