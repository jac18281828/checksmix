//! `TrapCode`, `FileHandle`, and the TRAP #0-#82 handlers behind them.

use super::MMix;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use tracing::debug;

/// The per-call byte bound shared by `Fopen`'s name, `Fwrite`, `Fputs` and
/// `Fputws` (`Fputws`'s own bound counted in wydes, `MAX_TRAP_WYDES`
/// below). `Fread` has no bound of its own: it transfers in chunks of at
/// most this size, stopping at `size` bytes, end of file or an error,
/// whichever comes first. **Departure from the reference:** none of
/// `Fopen`, `Fwrite`, `Fputs` or `Fputws` has one there. A fixed constant,
/// read from no register.
pub(super) const MAX_TRAP_BYTES: usize = 1_048_576;

/// `Fputws`'s bound in wydes: the same byte budget as `MAX_TRAP_BYTES`,
/// counted two bytes at a time.
const MAX_TRAP_WYDES: usize = MAX_TRAP_BYTES / 2;

/// TRAP code identifiers for MMIX, numbered per the MMIXAL reference: every
/// call is `TRAP 0,Code,Z`. `Z` means a different thing per code: ignored
/// for `Halt`; the handle (0-255) for the file calls and `Fputc`; the unit
/// for `Time` (0 seconds, 1 milliseconds, 2 microseconds); and for `Debug`,
/// the 0-based index, in program order, of a `debug "text"` directive,
/// looked up in the table `set_debug_strings` installed. `$255` carries any
/// further argument (an address, for a call that takes two).
/// `Fputc`, `Time` and `Debug` are checksmix's own extensions, given codes
/// (`#80`-`#82`) well above the reference's range so an old binary's codes
/// 11-13 reach the unhandled-TRAP diagnostic rather than the wrong call.
///
/// More trap codes may be added in future releases, so downstream matches
/// must carry a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum TrapCode {
    Halt = 0,
    Fopen = 1,
    Fclose = 2,
    Fread = 3,
    Fgets = 4,
    Fgetws = 5,
    Fwrite = 6,
    Fputs = 7,
    Fputws = 8,
    Fseek = 9,
    Ftell = 10,
    Fputc = 0x80,
    Time = 0x81,
    Debug = 0x82,
}

impl TrapCode {
    /// Convert a u8 to a TrapCode variant
    pub fn from_u8(n: u8) -> Option<Self> {
        match n {
            0 => Some(TrapCode::Halt),
            1 => Some(TrapCode::Fopen),
            2 => Some(TrapCode::Fclose),
            3 => Some(TrapCode::Fread),
            4 => Some(TrapCode::Fgets),
            5 => Some(TrapCode::Fgetws),
            6 => Some(TrapCode::Fwrite),
            7 => Some(TrapCode::Fputs),
            8 => Some(TrapCode::Fputws),
            9 => Some(TrapCode::Fseek),
            10 => Some(TrapCode::Ftell),
            0x80 => Some(TrapCode::Fputc),
            0x81 => Some(TrapCode::Time),
            0x82 => Some(TrapCode::Debug),
            _ => None,
        }
    }
}

/// One of `checksmix`'s open TRAP handles. Handles 0-2 are the standard
/// streams: no backing `File` and fixed capabilities. Fd 1 and 2 writes
/// route through the installed [`Host`]; a fd 0 read always fails, since
/// `Host` has no read primitive. Handles 3-255 are whatever `Fopen`'s mode
/// granted.
///
/// `read`, `write` and `seek` gate `Fread`/`Fgets`/`Fgetws`,
/// `Fwrite`/`Fputs`/`Fputc`/`Fputws`, and `Fseek`/`Ftell` respectively.
/// `read_write` marks a handle opened `BinaryReadWrite`: on such a handle a
/// read clears `write` and a write clears `read`, and `Fseek` restores both
/// — the reference's read-write switching rule. A handle opened
/// `BinaryRead`/`BinaryWrite` carries `seek` without `read_write`, so its
/// single capability never toggles.
pub(super) struct FileHandle {
    pub(super) file: Option<File>,
    pub(super) read: bool,
    pub(super) write: bool,
    pub(super) seek: bool,
    pub(super) read_write: bool,
}

impl MMix {
    /// Handle TRAP system calls
    /// Returns true if execution should continue, false if halted
    pub(super) fn handle_trap(&mut self, trap_code: TrapCode, arg: u8) -> bool {
        let arg255 = self.get_register(255);
        let result = match trap_code {
            TrapCode::Halt => self.handle_halt(arg),
            TrapCode::Fopen => self.handle_fopen(arg),
            TrapCode::Fclose => self.handle_fclose(arg),
            TrapCode::Fread => self.handle_fread(arg),
            TrapCode::Fgets => self.handle_fgets(arg),
            TrapCode::Fgetws => self.handle_fgetws(arg),
            TrapCode::Fwrite => self.handle_fwrite(arg),
            TrapCode::Fputs => self.handle_fputs(arg),
            TrapCode::Fputc => self.handle_fputc(arg),
            TrapCode::Fputws => self.handle_fputws(arg),
            TrapCode::Fseek => self.handle_fseek(arg),
            TrapCode::Ftell => self.handle_ftell(arg),
            TrapCode::Time => self.handle_time(arg),
            TrapCode::Debug => self.handle_debug(arg),
        };
        let result255 = self.get_register(255);
        self.host.trap(trap_code, arg, arg255, result255);
        result
    }

    /// TRAP 0: Halt - stop execution
    /// Parameter in $255: exit code (though typically ignored, could use Z parameter)
    fn handle_halt(&mut self, _arg: u8) -> bool {
        debug!("TRAP: Halt");
        let exit_code = self.get_register(255);
        self.exit_code = exit_code;
        // Halt is the machine's last chance to signal the host: the caller
        // exits without running destructors, so buffered output from earlier
        // Fputs/Fputc/Fputws calls would otherwise be discarded.
        self.host.flush();
        self.host.diagnostic(&format!(
            "HALT trap at PC={:#018x}, exit code={}",
            self.pc, exit_code
        ));
        self.advance_pc();
        false
    }

    /// Whether `handle` is open and grants read access.
    fn handle_readable(&self, handle: u8) -> bool {
        self.file_handles.get(&handle).is_some_and(|h| h.read)
    }

    /// Whether `handle` is open and grants write access.
    fn handle_writable(&self, handle: u8) -> bool {
        self.file_handles.get(&handle).is_some_and(|h| h.write)
    }

    /// Whether `handle` is open and grants seek access.
    fn handle_seekable(&self, handle: u8) -> bool {
        self.file_handles.get(&handle).is_some_and(|h| h.seek)
    }

    /// Fails a TRAP whose precondition `ok` does not hold: stores `failure`
    /// in `$255`, advances the PC, and reports `true` so the caller returns
    /// immediately. `false` when `ok` holds and the call proceeds.
    fn fail_unless(&mut self, ok: bool, failure: i64) -> bool {
        if !ok {
            self.set_register(255, failure as u64);
            self.advance_pc();
        }
        !ok
    }

    /// A read on a `BinaryReadWrite` handle clears its write capability
    /// until `Fseek` restores both.
    fn note_read(&mut self, handle: u8) {
        if let Some(entry) = self.file_handles.get_mut(&handle)
            && entry.read_write
        {
            entry.write = false;
        }
    }

    /// A write on a `BinaryReadWrite` handle clears its read capability
    /// until `Fseek` restores both.
    fn note_write(&mut self, handle: u8) {
        if let Some(entry) = self.file_handles.get_mut(&handle)
            && entry.read_write
        {
            entry.read = false;
        }
    }

    /// `Fseek` restores both capabilities on a `BinaryReadWrite` handle.
    fn note_seek(&mut self, handle: u8) {
        if let Some(entry) = self.file_handles.get_mut(&handle)
            && entry.read_write
        {
            entry.read = true;
            entry.write = true;
        }
    }

    /// Fails `Fopen` on a non-standard handle: closes it (a failed open
    /// leaves it closed, whether or not it was open before), sets `$255`
    /// to -1, advances the PC, and reports the call handled. Every
    /// `Fopen` failure past the standard-handle check goes through here,
    /// so none can leave a stale handle open. Handles 0-2 never reach
    /// this: `Fopen`/`Fclose` leave them exactly as they were.
    fn fail_fopen(&mut self, handle: u8) -> bool {
        self.file_handles.remove(&handle);
        self.set_register(255, (-1i64) as u64);
        self.advance_pc();
        true
    }

    /// Reads `Fopen`'s name argument: the guest's bytes at `name_addr` up to
    /// their terminating zero, passed through unchanged, capped at
    /// `MAX_TRAP_BYTES`. Returns `None`, having already logged the reason
    /// through `debug!`, when no zero falls within the bound or the bytes
    /// are not valid UTF-8.
    fn read_fopen_name(&self, handle: u8, name_addr: u64) -> Option<String> {
        let (name_bytes, truncated) = self.read_bounded_bytes(name_addr, MAX_TRAP_BYTES);
        if truncated {
            debug!(handle, "TRAP: Fopen name has no zero within the byte bound");
            return None;
        }
        match String::from_utf8(name_bytes) {
            Ok(filename) => Some(filename),
            Err(_) => {
                debug!(handle, "TRAP: Fopen name is not valid UTF-8");
                None
            }
        }
    }

    /// TRAP 1: Fopen. `Z` is the handle the caller chooses, 0-255; `$255`
    /// addresses a two-octa block holding the name address and the mode.
    /// Handles 0-2 belong to the host and always fail, leaving the stream
    /// as it was. Opening a handle already open closes it first; on
    /// failure the handle is left closed.
    ///
    /// The name is the guest's bytes up to its terminating zero, passed to
    /// the host unchanged, capped at `MAX_TRAP_BYTES`. A name with no zero
    /// within the bound, or one that is not valid UTF-8, fails with -1 and
    /// touches no file; every such failure logs through
    /// `debug!` only, like any other `Fopen` failure. **Departure from the
    /// reference:** a name that is valid bytes on the host's filesystem but
    /// not UTF-8, such as a Latin-1 name on a Linux filesystem, cannot be
    /// opened.
    fn handle_fopen(&mut self, handle: u8) -> bool {
        if handle <= 2 {
            debug!(handle, "TRAP: Fopen rejects a standard handle");
            self.set_register(255, (-1i64) as u64);
            self.advance_pc();
            return true;
        }

        let param_addr = self.get_register(255);
        let name_addr = self.read_octa(param_addr);
        let mode_octa = self.read_octa(param_addr.wrapping_add(8));

        let Some(filename) = self.read_fopen_name(handle, name_addr) else {
            return self.fail_fopen(handle);
        };

        debug!(handle, filename = %filename, mode = mode_octa, "TRAP: Fopen");

        if mode_octa > 4 {
            debug!(mode = mode_octa, "Invalid file open mode");
            return self.fail_fopen(handle);
        }
        let mode = mode_octa as u8;

        // TextRead=0, TextWrite=1, BinaryRead=2, BinaryWrite=3,
        // BinaryReadWrite=4: (read, write, seek, read_write).
        let caps = match mode {
            0 => (true, false, false, false),
            1 => (false, true, false, false),
            2 => (true, false, true, false),
            3 => (false, true, true, false),
            _ => (true, true, true, true), // mode == 4, checked above
        };

        // Opening an already-open handle closes it first.
        self.file_handles.remove(&handle);

        let opened = match mode {
            0 | 2 => OpenOptions::new().read(true).open(&filename),
            1 | 3 => OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&filename),
            _ => OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&filename),
        };

        match opened {
            Ok(file) => {
                let (read, write, seek, read_write) = caps;
                self.file_handles.insert(
                    handle,
                    FileHandle {
                        file: Some(file),
                        read,
                        write,
                        seek,
                        read_write,
                    },
                );
                self.set_register(255, 0);
                debug!(handle, "File opened successfully");
                self.advance_pc();
                true
            }
            Err(_) => {
                debug!(handle, "File open failed");
                self.fail_fopen(handle)
            }
        }
    }

    /// TRAP 2: Fclose. `Z` is the handle. Handles 0-2 belong to the host and
    /// always fail.
    fn handle_fclose(&mut self, handle: u8) -> bool {
        debug!(handle, "TRAP: Fclose");
        if handle <= 2 {
            self.set_register(255, (-1i64) as u64);
            self.advance_pc();
            return true;
        }
        match self.file_handles.remove(&handle) {
            Some(_) => self.set_register(255, 0),
            None => self.set_register(255, (-1i64) as u64),
        }
        self.advance_pc();
        true
    }

    /// TRAP 3: Fread. `Z` is the handle; `$255` addresses a two-octa block
    /// holding the buffer address and the byte count, read as a full
    /// octabyte so no target narrows it. Reads in chunks of at most
    /// `MAX_TRAP_BYTES`, so no allocation scales with the guest's
    /// request; the loop ends at the guest's requested size, end of file,
    /// or an I/O error, whichever comes first, and reports a mid-read
    /// error the same as an early EOF — the short-read count, not the
    /// all-or-nothing failure value. Every result is mod 2^64.
    fn handle_fread(&mut self, handle: u8) -> bool {
        let param_addr = self.get_register(255);
        let buffer_addr = self.read_octa(param_addr);
        let size = self.read_octa(param_addr.wrapping_add(8));

        debug!(
            handle,
            buffer_addr = format!("0x{:X}", buffer_addr),
            size,
            "TRAP: Fread"
        );

        // Handle 0 (StdIn) has no host read primitive.
        if self.fail_unless(
            self.handle_readable(handle) && handle != 0,
            u64::MAX.wrapping_sub(size) as i64,
        ) {
            return true;
        }
        self.note_read(handle);

        // Sized to what this call could possibly need: a size below the
        // cap allocates and zeroes only that many bytes, so many small
        // reads don't each pay for a 1 MiB buffer they never fill.
        let chunk_size = size.min(MAX_TRAP_BYTES as u64) as usize;
        let mut chunk = vec![0u8; chunk_size];
        let mut total: u64 = 0;
        let mut had_error = false;
        while total < size {
            let want = (size - total).min(chunk_size as u64) as usize;
            let file = self.open_file(handle);
            match file.read(&mut chunk[..want]) {
                Ok(0) => break,
                Ok(n) => {
                    for (i, &byte) in chunk[..n].iter().enumerate() {
                        self.write_byte(
                            buffer_addr.wrapping_add(total.wrapping_add(i as u64)),
                            byte,
                        );
                    }
                    total += n as u64;
                }
                Err(_) => {
                    had_error = true;
                    break;
                }
            }
        }

        let result = if had_error && total == 0 {
            u64::MAX.wrapping_sub(size)
        } else {
            total.wrapping_sub(size)
        };
        self.set_register(255, result);
        self.advance_pc();
        true
    }

    /// TRAP 4: Fgets. `Z` is the handle; `$255` addresses a two-octa block
    /// holding the buffer address and the buffer size. Reads until `size -
    /// 1` characters or a newline, then a zero byte; returns the count
    /// stored, or -1 when `size` is 0 or end of file/an error comes before
    /// any character.
    fn handle_fgets(&mut self, handle: u8) -> bool {
        let param_addr = self.get_register(255);
        let buffer_addr = self.read_octa(param_addr);
        let max_size = self.read_octa(param_addr.wrapping_add(8)) as usize;

        debug!(
            handle,
            buffer_addr = format!("0x{:X}", buffer_addr),
            max_size,
            "TRAP: Fgets"
        );

        let ok = max_size != 0 && self.handle_readable(handle) && handle != 0;
        if self.fail_unless(ok, -1) {
            return true;
        }
        self.note_read(handle);

        let mut count = 0usize;
        while count < max_size - 1 {
            let mut byte = [0u8; 1];
            let file = self.open_file(handle);
            match file.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    self.write_byte(buffer_addr.wrapping_add(count as u64), byte[0]);
                    count += 1;
                    if byte[0] == b'\n' {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        self.write_byte(buffer_addr.wrapping_add(count as u64), 0);
        self.set_register(
            255,
            if count == 0 {
                (-1i64) as u64
            } else {
                count as u64
            },
        );
        self.advance_pc();
        true
    }

    /// TRAP 5: Fgetws. `Z` is the handle; `$255` addresses a two-octa block
    /// holding the buffer address (rounded down to even) and the buffer
    /// size in wydes. Wydes are read raw, two bytes each in memory order;
    /// stops at the wyde `#000A`, `size - 1` wydes, end of file, or an
    /// error, then stores a zero wyde. Returns the wyde count, or -1 when
    /// `size` is 0 or nothing was read.
    fn handle_fgetws(&mut self, handle: u8) -> bool {
        let param_addr = self.get_register(255);
        let buffer_addr = self.read_octa(param_addr) & !1u64;
        let max_wydes = self.read_octa(param_addr.wrapping_add(8)) as usize;

        debug!(
            handle,
            buffer_addr = format!("0x{:X}", buffer_addr),
            max_wydes,
            "TRAP: Fgetws"
        );

        let ok = max_wydes != 0 && self.handle_readable(handle) && handle != 0;
        if self.fail_unless(ok, -1) {
            return true;
        }
        self.note_read(handle);

        let mut count = 0usize;
        while count < max_wydes - 1 {
            let mut wyde = [0u8; 2];
            let file = self.open_file(handle);
            match file.read_exact(&mut wyde) {
                Ok(_) => {
                    self.write_byte(buffer_addr.wrapping_add((count * 2) as u64), wyde[0]);
                    self.write_byte(buffer_addr.wrapping_add((count * 2 + 1) as u64), wyde[1]);
                    count += 1;
                    if wyde == [0x00, 0x0A] {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        self.write_byte(buffer_addr.wrapping_add((count * 2) as u64), 0);
        self.write_byte(buffer_addr.wrapping_add((count * 2 + 1) as u64), 0);
        self.set_register(
            255,
            if count == 0 {
                (-1i64) as u64
            } else {
                count as u64
            },
        );
        self.advance_pc();
        true
    }

    /// TRAP 6: Fwrite. `Z` is the handle; `$255` addresses a two-octa block
    /// holding the buffer address and the byte count, read as a full
    /// octabyte so no target narrows it. Moves at most `MAX_TRAP_BYTES`
    /// bytes a call, a larger `size` written and reported as a short
    /// write (departure from the reference, which streams the full
    /// size). Writes in a loop, so a short underlying write is reflected
    /// in the result rather than masked. Returns 0 if all `size` bytes
    /// were written, else `n - size` mod 2^64 for the `n` bytes actually
    /// written (`-size` mod 2^64 if the handle lacks write access, `n`
    /// then being 0).
    fn handle_fwrite(&mut self, handle: u8) -> bool {
        let param_addr = self.get_register(255);
        let buffer_addr = self.read_octa(param_addr);
        let size = self.read_octa(param_addr.wrapping_add(8));

        debug!(
            handle,
            buffer_addr = format!("0x{:X}", buffer_addr),
            size,
            "TRAP: Fwrite"
        );

        if self.fail_unless(self.handle_writable(handle), 0u64.wrapping_sub(size) as i64) {
            return true;
        }
        self.note_write(handle);

        let capped = size.min(MAX_TRAP_BYTES as u64) as usize;
        let mut buffer = Vec::with_capacity(capped);
        for i in 0..capped {
            buffer.push(self.read_byte(buffer_addr.wrapping_add(i as u64)));
        }

        let written = match handle {
            1 | 2 => match self.host.write(handle, &buffer) {
                Ok(_) => buffer.len(),
                Err(_) => 0,
            },
            _ => {
                let file = self.open_file(handle);
                let mut total = 0usize;
                while total < buffer.len() {
                    match file.write(&buffer[total..]) {
                        Ok(0) => break,
                        Ok(n) => total += n,
                        Err(_) => break,
                    }
                }
                total
            }
        };

        let result = (written as u64).wrapping_sub(size);
        self.set_register(255, result);
        self.advance_pc();
        true
    }

    /// Read a NUL-terminated byte string from memory starting at `addr`.
    /// Bytes are returned verbatim (no UTF-8 widening). The second element
    /// is true when `max_len` bytes were read without finding a zero: a
    /// string of exactly `max_len` bytes, followed by its zero, is not too
    /// long, and the second element is false whenever the zero is found,
    /// whatever the string's length. Walks memory using wrapping
    /// arithmetic so `addr` near `u64::MAX` cannot panic.
    fn read_bounded_bytes(&self, addr: u64, max_len: usize) -> (Vec<u8>, bool) {
        let mut bytes = Vec::new();
        let mut cur = addr;
        loop {
            let byte = self.read_byte(cur);
            if byte == 0 {
                return (bytes, false);
            }
            if bytes.len() == max_len {
                return (bytes, true);
            }
            bytes.push(byte);
            cur = cur.wrapping_add(1);
        }
    }

    /// The open `File` behind `handle`, for a read/write/seek call past its
    /// capability check. Infallible there: `handle_readable`/
    /// `handle_writable`/`handle_seekable` already confirmed an entry
    /// exists, and every caller either excludes handle 0/1/2 (whose entries
    /// carry no `File`) or, for a read, has already turned handle 0 aside
    /// before reaching here.
    fn open_file(&mut self, handle: u8) -> &mut File {
        self.file_handles
            .get_mut(&handle)
            .and_then(|entry| entry.file.as_mut())
            .expect("capability check already confirmed handle is open with a real file")
    }

    /// Write raw bytes to the destination identified by an MMIX file
    /// descriptor: 1 and 2 go through the installed `Host` (locked
    /// stdout/stderr writes for `StdHost`); anything else is looked up in
    /// `file_handles`, whose retry and locking behavior is `std::fs::File`'s.
    fn write_bytes_to_fd(&mut self, fd: u8, bytes: &[u8]) -> std::io::Result<()> {
        match fd {
            1 | 2 => self.host.write(fd, bytes),
            _ => match self.file_handles.get_mut(&fd).and_then(|h| h.file.as_mut()) {
                Some(file) => file.write_all(bytes),
                None => Err(std::io::Error::other("file descriptor not open")),
            },
        }
    }

    /// TRAP 7: Fputs. `Z` is the handle; `$255` is the string address.
    /// Writes bytes up to, not including, the first zero byte, with no
    /// byte value translated. **Departure from the reference:** capped at
    /// `MAX_TRAP_BYTES` per call; a longer string writes that many bytes,
    /// reports a diagnostic, and returns the count actually written.
    /// Returns the byte count written, or -1.
    fn handle_fputs(&mut self, handle: u8) -> bool {
        let str_addr = self.get_register(255);
        let (bytes, truncated) = self.read_bounded_bytes(str_addr, MAX_TRAP_BYTES);
        if truncated {
            self.host
                .diagnostic("Warning: Fputs string too long, truncating");
        }
        debug!(
            handle,
            str_addr = format!("0x{:X}", str_addr),
            "TRAP: Fputs"
        );

        if self.fail_unless(self.handle_writable(handle), -1) {
            return true;
        }
        self.note_write(handle);

        match self.write_bytes_to_fd(handle, &bytes) {
            Ok(_) => self.set_register(255, bytes.len() as u64),
            Err(_) => {
                debug!(handle, "Fputs write failed");
                self.set_register(255, (-1i64) as u64);
            }
        }
        self.advance_pc();
        true
    }

    /// TRAP #80: Fputc, checksmix's own extension. `Z` is the handle;
    /// `$255`'s low byte is the character. Shares `Fputs`'s write-capability
    /// check and read-write switching. Returns 0 on success, or -1.
    fn handle_fputc(&mut self, handle: u8) -> bool {
        let ch = (self.get_register(255) & 0xFF) as u8;
        debug!(handle, ch = format!("0x{:02X}", ch), "TRAP: Fputc");

        if self.fail_unless(self.handle_writable(handle), -1) {
            return true;
        }
        self.note_write(handle);

        match self.write_bytes_to_fd(handle, &[ch]) {
            Ok(_) => self.set_register(255, 0),
            Err(_) => {
                debug!(handle, "Fputc write failed");
                self.set_register(255, (-1i64) as u64);
            }
        }
        self.advance_pc();
        true
    }

    /// TRAP 8: Fputws. `Z` is the handle; `$255` is the string address.
    /// Wyde characters, two bytes each in memory order, written up to, not
    /// including, the first zero wyde. **Departure from the reference:**
    /// capped at `MAX_TRAP_WYDES` per call, `MAX_TRAP_BYTES`'s budget in
    /// wydes; a longer string writes that many wydes, reports a diagnostic,
    /// and returns the count actually written. A string of exactly
    /// `MAX_TRAP_WYDES` wydes, followed by its zero wyde, is not too long.
    /// Returns the wyde count written, or -1.
    fn handle_fputws(&mut self, handle: u8) -> bool {
        let str_addr = self.get_register(255);
        let mut bytes = Vec::new();
        let mut addr = str_addr;
        let mut wyde_count = 0usize;
        loop {
            let hi = self.read_byte(addr);
            let lo = self.read_byte(addr.wrapping_add(1));
            if hi == 0 && lo == 0 {
                break;
            }
            if wyde_count == MAX_TRAP_WYDES {
                self.host
                    .diagnostic("Warning: Fputws string too long, truncating");
                break;
            }
            bytes.push(hi);
            bytes.push(lo);
            wyde_count += 1;
            addr = addr.wrapping_add(2);
        }
        debug!(
            handle,
            str_addr = format!("0x{:X}", str_addr),
            "TRAP: Fputws"
        );

        if self.fail_unless(self.handle_writable(handle), -1) {
            return true;
        }
        self.note_write(handle);

        match self.write_bytes_to_fd(handle, &bytes) {
            Ok(_) => self.set_register(255, wyde_count as u64),
            Err(_) => {
                debug!(handle, "Fputws write failed");
                self.set_register(255, (-1i64) as u64);
            }
        }
        self.advance_pc();
        true
    }

    /// TRAP 9: Fseek. `Z` is the handle; `$255` is the offset. `offset >= 0`
    /// positions `offset` bytes from the start; `offset < 0` positions
    /// `-offset - 1` bytes before the end. On a `BinaryReadWrite` handle,
    /// restores both read and write capability. Returns 0, or -1.
    fn handle_fseek(&mut self, handle: u8) -> bool {
        let offset = self.get_register(255) as i64;
        debug!(handle, offset, "TRAP: Fseek");

        if self.fail_unless(self.handle_seekable(handle), -1) {
            return true;
        }
        self.note_seek(handle);

        let seek_from = if offset >= 0 {
            SeekFrom::Start(offset as u64)
        } else {
            SeekFrom::End(offset + 1)
        };

        let file = self.open_file(handle);
        match file.seek(seek_from) {
            Ok(pos) => {
                self.set_register(255, 0);
                debug!(pos, "Seek successful");
            }
            Err(_) => {
                self.set_register(255, (-1i64) as u64);
                debug!("Seek failed");
            }
        }
        self.advance_pc();
        true
    }

    /// TRAP 10: Ftell. `Z` is the handle. Returns the current position, or
    /// -1.
    fn handle_ftell(&mut self, handle: u8) -> bool {
        debug!(handle, "TRAP: Ftell");

        if self.fail_unless(self.handle_seekable(handle), -1) {
            return true;
        }

        let file = self.open_file(handle);
        match file.stream_position() {
            Ok(pos) => {
                self.set_register(255, pos);
                debug!(pos, "Ftell successful");
            }
            Err(_) => {
                self.set_register(255, (-1i64) as u64);
                debug!("Ftell failed");
            }
        }
        self.advance_pc();
        true
    }

    /// TRAP #82: Debug, checksmix's own extension backing the `debug
    /// "text"` directive. `Z` indexes the table `set_debug_strings`
    /// installed; writes that string and a newline to handle 1 through
    /// `Host::write`. Changes no register, `$255` included. An index past
    /// the table's end reports a diagnostic and continues.
    fn handle_debug(&mut self, index: u8) -> bool {
        match self.debug_strings.get(index as usize) {
            Some(text) => {
                let mut bytes = text.clone();
                bytes.push(b'\n');
                if let Err(err) = self.host.write(1, &bytes) {
                    self.host
                        .diagnostic(&format!("debug: write to handle 1 failed: {err}"));
                }
            }
            None => {
                self.host
                    .diagnostic(&format!("debug: index {index} has no string in the table"));
            }
        }
        self.advance_pc();
        true
    }

    fn handle_time(&mut self, unit: u8) -> bool {
        let micros = self.host.now_micros();

        let time_value = match unit {
            0 => micros / 1_000_000, // Seconds (default)
            1 => micros / 1_000,     // Milliseconds
            2 => micros,             // Microseconds
            _ => {
                debug!("Invalid time unit: {}", unit);
                0
            }
        };
        debug!("TRAP: Time (unit={}) => {}", unit, time_value);
        self.set_register(255, time_value);
        self.advance_pc();
        true
    }
}
