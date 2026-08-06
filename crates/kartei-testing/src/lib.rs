//! The test harness, and the conditions every suite in this workspace is held
//! to.
//!
//! The conditions are written down in `docs/testing.md`. Two of them are the
//! ones a suite drifts away from silently rather than loudly, so this crate
//! makes the compliant thing the easy thing:
//!
//! A test gets its own temporary directory, created by the test and removed
//! when it ends, never a shared well known path. [`TempDir`] is that.
//!
//! A test that needs a port binds zero and reads back what the operating system
//! assigned, never a fixed number. [`bind_ephemeral`] is that.
//!
//! The third helper is a query rather than a convenience. [`is_elevated`] asks
//! the operating system whether this process is running with administrative
//! rights, so a suite that starts to need them reds instead of drifting into
//! requiring them on the one machine where somebody happened to run it that
//! way.
//!
//! This crate has no dependencies, and that is a property of the harness rather
//! than an accident. Anything it pulled in would sit under every suite in the
//! workspace, and the two platform queries it needs are two function
//! declarations rather than a crate.

use std::fs;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// A directory that belongs to one test and is removed when that test ends.
///
/// The path is unique, and unique by construction rather than by hope: the name
/// carries the process id, a nanosecond timestamp and a counter, and the
/// directory is created with [`fs::create_dir`], which fails rather than
/// succeeds if the name is already taken. A collision therefore becomes another
/// attempt, never a second test writing into the first test's directory.
///
/// That is the failure this type exists to prevent. Two tests sharing one well
/// known path fail intermittently, in whichever order the scheduler picks, and
/// the failure looks like a bug in the code under test rather than in the
/// harness.
pub struct TempDir {
    path: PathBuf,
}

/// Counter behind the uniqueness of a temporary directory name.
static NEXT: AtomicU64 = AtomicU64::new(0);

impl TempDir {
    /// Creates a directory for the calling test.
    ///
    /// `label` is for a human reading a directory listing after a crash. Only
    /// ASCII letters, digits, `-` and `_` survive into the name; everything else
    /// becomes `_`, so a label taken from a test name cannot reach outside the
    /// directory it is supposed to name.
    ///
    /// Panics rather than returning an error. A harness that cannot give a test
    /// its own directory has not found a condition the test should handle, it
    /// has found a broken machine, and every caller would write the same
    /// `unwrap`.
    #[must_use]
    pub fn new(label: &str) -> TempDir {
        let safe: String = label
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        let parent = std::env::temp_dir();
        let pid = std::process::id();

        // Bounded rather than unbounded: a machine that cannot produce a free
        // name in this many tries has something wrong with it that retrying will
        // not fix, and an unbounded loop would hang the suite instead of
        // reporting it.
        for _ in 0..64 {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!("kartei-{safe}-{pid}-{nanos}-{n}"));

            match fs::create_dir(&path) {
                Ok(()) => return TempDir { path },
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!(
                    "could not create a temporary directory at {}: {e}",
                    path.display()
                ),
            }
        }

        panic!(
            "could not find an unused temporary directory name under {} in 64 attempts",
            parent.display()
        );
    }

    /// The directory. It exists for as long as this value does.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Reported rather than asserted. A panic here would replace the real
        // failure of a test that is already unwinding, and on Windows a file
        // another process still holds open is a routine reason for this to fail.
        if let Err(e) = fs::remove_dir_all(&self.path) {
            eprintln!(
                "could not remove the temporary directory {}: {e}",
                self.path.display()
            );
        }
    }
}

/// Binds a port the operating system chooses, on the loopback interface.
///
/// Returns the bound listener rather than a bare port number, and that is the
/// whole point of the helper. Binding zero, reading the port back and closing
/// the socket hands out a number that another process can take before the
/// caller uses it, which is the same intermittent failure as a fixed port with
/// a longer fuse. Holding the listener is what makes the port the caller's.
///
/// Loopback only. A test does not reach the network, and a listener on
/// `0.0.0.0` is reachable from one.
#[must_use]
pub fn bind_ephemeral() -> TcpListener {
    TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).unwrap_or_else(|e| {
        panic!("could not bind an ephemeral port on the loopback interface: {e}")
    })
}

/// The port a listener was given.
#[must_use]
pub fn port_of(listener: &TcpListener) -> u16 {
    listener
        .local_addr()
        .unwrap_or_else(|e| panic!("a bound listener could not report its own address: {e}"))
        .port()
}

/// Whether this process is running with administrative rights.
///
/// Asks the operating system rather than guessing from the environment. On Unix
/// that is the effective user id; on Windows it is the elevation flag on the
/// process token, which is the only answer that distinguishes an elevated
/// process from an administrator account running unelevated.
#[must_use]
pub fn is_elevated() -> bool {
    platform::is_elevated()
}

/// Fails the calling test if the suite is running with administrative rights.
///
/// A suite that is only ever run elevated acquires tests that need elevation
/// without anyone deciding to allow it, and the cost of that decision is paid
/// by every contributor and every machine afterwards. This turns the drift into
/// a failure at the moment it happens.
pub fn assert_not_elevated() {
    assert!(
        !is_elevated(),
        "this suite is running with administrative rights. No test in this \
         workspace may require them, so a run that has them can pass a test \
         that would fail for everybody else. Run the suite as an ordinary user."
    );
}

#[cfg(unix)]
mod platform {
    /// The effective user id. Root is 0 and nothing else is.
    pub(super) fn is_elevated() -> bool {
        // SAFETY: geteuid takes no arguments, touches no memory the caller owns,
        // cannot fail and is always present in libc, which std has already
        // linked.
        unsafe { geteuid() == 0 }
    }

    unsafe extern "C" {
        fn geteuid() -> u32;
    }
}

#[cfg(windows)]
mod platform {
    use core::ffi::c_void;
    use core::ptr;

    /// `TOKEN_QUERY`, the only access this needs to the process token.
    const TOKEN_QUERY: u32 = 0x0008;
    /// `TokenElevation` in the `TOKEN_INFORMATION_CLASS` enumeration.
    const TOKEN_ELEVATION: i32 = 20;

    /// The `TOKEN_ELEVATION` structure: one non zero field when elevated.
    #[repr(C)]
    struct Elevation {
        token_is_elevated: u32,
    }

    pub(super) fn is_elevated() -> bool {
        let mut token: *mut c_void = ptr::null_mut();

        // SAFETY: GetCurrentProcess returns a pseudo handle that needs no
        // closing, and `token` is a live local for the whole call.
        let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut token) };
        assert!(
            opened != 0,
            "could not open this process's access token, so whether the suite \
             is elevated is unknown. That is not the same as knowing it is not, \
             so this fails rather than assuming."
        );

        let mut elevation = Elevation {
            token_is_elevated: 0,
        };
        let mut returned: u32 = 0;

        // SAFETY: `token` is the handle just opened, the buffer and the length
        // describe the same live local, and `returned` is a live local.
        let queried = unsafe {
            GetTokenInformation(
                token,
                TOKEN_ELEVATION,
                (&raw mut elevation).cast::<c_void>(),
                u32::try_from(size_of::<Elevation>()).expect("the structure is four bytes"),
                &raw mut returned,
            )
        };

        // SAFETY: `token` came from OpenProcessToken and is not used again.
        unsafe { CloseHandle(token) };

        assert!(
            queried != 0,
            "could not read the elevation flag from this process's access \
             token, so whether the suite is elevated is unknown."
        );

        elevation.token_is_elevated != 0
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn OpenProcessToken(process: *mut c_void, desired: u32, token: *mut *mut c_void) -> i32;
        fn GetTokenInformation(
            token: *mut c_void,
            class: i32,
            information: *mut c_void,
            length: u32,
            returned: *mut u32,
        ) -> i32;
    }
}
