//! The `Host` trait, its `Box` impl, and `StdHost`.

use super::TrapCode;
use std::any::Any;
use std::io::{Write, stderr, stdout};
use std::time::SystemTime;

/// Routes every process-level effect an `MMix` produces: writes to fd 1/2,
/// the wall clock, and diagnostic messages (`StdHost` prints them to
/// stderr).
///
/// `MMix::new()` installs `StdHost`, which writes to the process's own
/// stdout and stderr. `MMix::with_host` accepts any `Host`, which is how an
/// embedder (a wasm host with no stdout, a test harness that wants to
/// inspect bytes rather than print them) captures what the machine emits
/// instead of losing it to the process.
///
/// File-descriptor traps (`Fopen`/`Fclose`/`Fread`/`Fgets`/`Fgetws`/
/// `Fwrite`/`Fseek`/`Ftell` on a handle above 2, and fd 3+ of `Fputs`/
/// `Fputc`/`Fputws`) do not go through the host — they keep using `std::fs`
/// directly and fail naturally on platforms without a filesystem. Handles
/// 0-2 belong to the host: `Fopen`/`Fclose` reject them. Fd 1 and 2 writes
/// route through `Host`; a fd 0 (`StdIn`) read always fails, since `Host`
/// has no read primitive.
///
/// `flush` and `trap` have no-op defaults, so an embedder implements only
/// what it needs. The trait is object-safe — `MMix` stores it as
/// `Box<dyn Host>` — and any method added in a future release will carry a
/// default, so implementors do not break.
///
/// `MMix::host_mut` and `MMix::into_host` reach a host moved in with
/// `with_host` again, by reference or by consuming the machine. A host that
/// wants its caller to read what it captured without borrowing the `MMix`
/// can instead hold its buffers behind a shared handle —
/// `Rc<RefCell<Vec<u8>>>` or similar — cloned *before* the host is moved
/// in, so the caller keeps one clone and the `MMix` owns the other.
///
/// Because `MMix` stores the host as `Box<dyn Host>`, it is none of `Send`,
/// `Sync`, `UnwindSafe`, or `RefUnwindSafe`. The intended embedders are
/// single-threaded — a browser playground, a test harness — and capture into
/// `Rc<RefCell<_>>`; see the [`MMix`] docs.
///
/// ```
/// use checksmix::{Host, MMix, TrapCode};
/// use std::cell::RefCell;
/// use std::rc::Rc;
///
/// struct Capture(Rc<RefCell<Vec<u8>>>);
///
/// impl Host for Capture {
///     fn write(&mut self, _fd: u8, bytes: &[u8]) -> std::io::Result<()> {
///         self.0.borrow_mut().extend_from_slice(bytes);
///         Ok(())
///     }
///     fn now_micros(&mut self) -> u64 { 0 }
///     fn diagnostic(&mut self, _msg: &str) {}
///     fn trap(&mut self, code: TrapCode, _arg: u8, _before: u64, _after: u64) {
///         // TrapCode is #[non_exhaustive]: a wildcard arm is required
///         match code {
///             TrapCode::Fputc => {}
///             _ => {}
///         }
///     }
/// }
///
/// // Keep one clone of the buffer; the machine owns the other.
/// let out = Rc::new(RefCell::new(Vec::new()));
/// let mut mmix = MMix::with_host(Capture(out.clone()));
///
/// mmix.set_register(255, u64::from(b'X'));
/// mmix.write_tetra(0, 0x00008001); // TRAP 0, Fputc (#80), fd 1
/// mmix.execute_instruction();
///
/// assert_eq!(&*out.borrow(), b"X");
/// ```
pub trait Host: Any {
    /// Write raw bytes to file descriptor `fd` (only 1 or 2 reach the
    /// host — see the trait docs). Returns `Ok(())` on success, matching
    /// `write_all` rather than reporting a partial-write count. On success
    /// `Fputs` and `Fputws` store `bytes.len()` in `$255`; `Fputc` stores 0.
    fn write(&mut self, fd: u8, bytes: &[u8]) -> std::io::Result<()>;

    /// Flush any buffered output. `Halt` is the only event that calls this,
    /// mirroring the process exiting without running destructors — a
    /// program that stops via register-form `TRAP` or the `TRIP`
    /// instruction never reaches it, and `MMix` has no `Drop`. A host must
    /// not rely on `flush` for correctness. No-op by default.
    fn flush(&mut self) {}

    /// The current time in microseconds since the Unix epoch. The only
    /// clock primitive `MMix` needs — `handle_time` derives seconds and
    /// milliseconds from it by division.
    fn now_micros(&mut self) -> u64;

    /// Report an operator-facing diagnostic message (an unhandled trap
    /// code, a truncated string, a HALT/TRIP notice). `StdHost` sends these
    /// to stderr via `eprintln!`.
    fn diagnostic(&mut self, msg: &str);

    /// Observe a trap after `handle_trap`'s dispatch has run, with `$255`
    /// captured both before and after. No-op by default.
    ///
    /// Only recognized trap codes reach this hook. An unhandled code, the
    /// register form of `TRAP` (`X != 0`, which halts the machine), and the
    /// `TRIP` instruction all report through `diagnostic` instead.
    ///
    /// `arg255` and `result255` do not mean the same thing for every trap:
    /// - `Halt` never writes `$255`, so `result255` is simply the exit code
    ///   the program supplied in `$255` before the trap, echoed back.
    /// - `Debug` never writes `$255` either, so `arg255` and `result255` are
    ///   both whatever `$255` happened to hold — before and after are equal.
    /// - `Time` takes its unit in `arg` (the Z operand), not in `$255`, so
    ///   `arg255` is stale on entry and only `result255` reflects the trap.
    /// - `Fputc` and `Fclose` leave a status in `result255` (0 on success,
    ///   `-1` as an unsigned octabyte on failure), not a count or a value.
    ///
    /// An embedder rendering a trap log should read these four cases
    /// before trusting `arg255`/`result255` at face value.
    fn trap(&mut self, code: TrapCode, arg: u8, arg255: u64, result255: u64) {
        let _ = (code, arg, arg255, result255);
    }
}

/// Lets a caller that already holds a boxed host — one selected at runtime
/// from several types, say — pass it to `MMix::with_host` directly.
///
/// There is deliberately no matching impl for `&mut H`. Both `Box` and `&mut`
/// are `#[fundamental]`, so downstream may implement `Host` for either and a
/// blanket impl added later could collide — but `with_host` takes `H: 'static`
/// by value, so a borrowed host is only usable when the borrow is `'static`,
/// which is rare enough not to buy the surface.
impl<H: Host + ?Sized> Host for Box<H> {
    fn write(&mut self, fd: u8, bytes: &[u8]) -> std::io::Result<()> {
        (**self).write(fd, bytes)
    }

    fn flush(&mut self) {
        (**self).flush()
    }

    fn now_micros(&mut self) -> u64 {
        (**self).now_micros()
    }

    fn diagnostic(&mut self, msg: &str) {
        (**self).diagnostic(msg)
    }

    fn trap(&mut self, code: TrapCode, arg: u8, arg255: u64, result255: u64) {
        (**self).trap(code, arg, arg255, result255)
    }
}

/// The `Host` behind `MMix::new()`: the process's own I/O — locked
/// `stdout`/`stderr` writes, `stdout().flush()` on halt, `SystemTime` for
/// the clock, and `eprintln!` for diagnostics.
///
/// `write_bytes_to_fd` only ever calls `Host::write` with fd 1 or 2 (fd 3+
/// reads its `File` from `file_handles`), but `StdHost` is a general `Host`
/// implementation, so it treats any other fd as an error rather than
/// assuming that invariant.
pub struct StdHost;

impl Host for StdHost {
    fn write(&mut self, fd: u8, bytes: &[u8]) -> std::io::Result<()> {
        match fd {
            1 => stdout().lock().write_all(bytes),
            2 => stderr().lock().write_all(bytes),
            _ => Err(std::io::Error::other(format!(
                "StdHost: unsupported file descriptor {fd}"
            ))),
        }
    }

    fn flush(&mut self) {
        let _ = stdout().flush();
    }

    fn now_micros(&mut self) -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }

    fn diagnostic(&mut self, msg: &str) {
        eprintln!("{msg}");
    }
}
