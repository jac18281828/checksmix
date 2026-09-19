//! The register stack: PUSHJ/PUSHGO frames, POP, and SAVE/UNSAVE contexts.

use super::{MMix, RA_MAX, SAVE_SPECIALS, SpecialReg};

/// Base of the register stack: `rO`/`rS` start here, and
/// [`MMix::call_depth`]'s walk stops here. `src/mmixal.rs` seeds the
/// `Stack_Segment` predefined symbol from this same constant.
pub(crate) const STACK_SEGMENT_START: u64 = 0x6000000000000000;

/// What [`MMix::pop_frame`] found at `rO`.
pub(super) enum PopFrame {
    /// A frame popped cleanly; the branch target follows.
    Frame(u64),
    /// `rO` was at or below the stack base — `POP`'s fallback.
    NoFrame,
    /// `rO` named an impossible placement; already rejected with a
    /// diagnostic and the machine left unchanged.
    Rejected,
}

/// Addresses below a saved context's packed octa, in the layout
/// [`MMix::save_context`] wrote: the twelve specials, the `rg_saved`
/// globals, the marker holding the saved local count, and the locals below
/// it.
struct SaveContextLayout {
    specials_base: u64,
    globals_base: u64,
    global_count: u64,
    local_count: u64,
    locals_base: u64,
}

impl MMix {
    /// Push a frame onto the register stack in memory and slide the window
    /// down.
    ///
    /// `PUSHJ $X` (`X < rG`) pushes `X+1` entries at the current `rO`:
    /// `M8[rO+8i] = $i` for `i < X`, and `M8[rO+8X] = X` — the hole, which
    /// `POP` reads back to know how much to restore. `rO` and `rS` both
    /// advance to `rO + 8(X+1)`; nothing else on the stack is written.
    /// `X ≥ rG` pushes all of `$0..$(rL-1)` instead, with `rL` as the hole.
    ///
    /// The live window then slides: caller's `$(X+1)..$(rL-1)` become
    /// callee's `$0..`, every vacated slot up through `rG-1` zeroes so a
    /// fresh local reads zero, and `rJ := pc + 4`.
    pub(super) fn push_frame(&mut self, x: u8) {
        let rg = self.get_special(SpecialReg::RG) as u8;
        let rl_old = self.get_special(SpecialReg::RL) as u8;
        // PUSHJ $X with X >= rG pushes all of $0..$(rL-1) and starts the
        // callee at rL = 0; the hole for the later POP is rL, not X.
        let x = if x >= rg { rl_old } else { x };
        let ro = self.get_special(SpecialReg::RO);

        for i in 0..x {
            let addr = ro.wrapping_add((i as u64).wrapping_mul(8));
            self.write_octa(addr, self.general_regs[i as usize]);
        }
        let hole_addr = ro.wrapping_add((x as u64).wrapping_mul(8));
        self.write_octa(hole_addr, x as u64);

        // Slide live window: caller's $(x+1)..$(rL-1) become callee's $0.. .
        let new_rl = rl_old.saturating_sub(x).saturating_sub(1);
        for i in 0..new_rl {
            let src = (x as usize) + 1 + (i as usize);
            self.general_regs[i as usize] = self.general_regs[src];
        }
        for i in (new_rl as usize)..(rg as usize) {
            self.general_regs[i] = 0;
        }

        let new_ro = ro.wrapping_add((x as u64 + 1).wrapping_mul(8));
        self.set_special(SpecialReg::RO, new_ro);
        self.set_special(SpecialReg::RS, new_ro);
        self.set_special(SpecialReg::RL, new_rl as u64);
        self.set_special(SpecialReg::RJ, self.pc.wrapping_add(4));
    }

    /// Why `rO` above [`STACK_SEGMENT_START`] cannot hold a frame: `None`
    /// when it is a plain in-segment address, otherwise the reason
    /// [`MMix::pop_frame`] rejects it with.
    fn ro_placement_violation(ro: u64) -> Option<&'static str> {
        if !ro.is_multiple_of(8) {
            Some("is misaligned")
        } else if ro >= 0x8000000000000000 {
            Some("is outside the register stack segment")
        } else {
            None
        }
    }

    /// Pop a frame from the register stack in memory and slide return
    /// values back. `rO` at or below [`STACK_SEGMENT_START`] means no
    /// frame remains: [`PopFrame::NoFrame`], the fallback `POP` takes.
    /// Any other placement outside the stack segment, or a misaligned
    /// `rO`, can only come from a forged [`MMix::set_special`] — the
    /// reference would raise a protection fault there, so this halts
    /// through [`MMix::reject`] instead: [`PopFrame::Rejected`].
    ///
    /// Otherwise the hole comes from memory: `x = M8[rO-8] mod 256`. The
    /// caller's `$0..$(x-1)` restore from `M8[rO-8(x+1)..]`, skipping any
    /// index at or above the current `rG`. `rO` and `rS` retract to
    /// `rO - 8(x+1)` — exactly where the matching `PUSHJ` found them. `rJ`
    /// is left unchanged.
    pub(super) fn pop_frame(&mut self, n: u8, yz: u16) -> PopFrame {
        let ro = self.get_special(SpecialReg::RO);
        if ro <= STACK_SEGMENT_START {
            return PopFrame::NoFrame;
        }
        if let Some(reason) = Self::ro_placement_violation(ro) {
            self.reject(&format!(
                "POP {n},{yz}: rO={ro:#018x} {reason}; the reference would \
                 raise a protection fault at PC={:#018x}",
                self.pc
            ));
            return PopFrame::Rejected;
        }

        let rg = self.get_special(SpecialReg::RG) as u8;
        let l = self.get_special(SpecialReg::RL) as u16;

        let x = (self.read_octa(ro.wrapping_sub(8)) % 256) as u8;

        // If X > L, X becomes L+1 and the hole gets zero regardless of what
        // the callee left there.
        let clamped = (n as u16) > l;
        let count = if clamped { l + 1 } else { n as u16 };

        // Snapshot the callee's $0..$(count-1) before the frame is torn down.
        let mut returns = [0u64; 256];
        returns[..count as usize].copy_from_slice(&self.general_regs[..count as usize]);

        let new_ro = ro.wrapping_sub((x as u64 + 1).wrapping_mul(8));

        // Every register from the new rL through rG-1 is marginal — clear
        // the whole local range before restoring or writing anything back.
        for i in 0..(rg as usize) {
            self.general_regs[i] = 0;
        }

        // Restore caller's $0..$(x-1) from memory, skipping any index at or
        // above the current rG.
        for i in 0..x {
            if (i as usize) < rg as usize {
                let addr = new_ro.wrapping_add((i as u64).wrapping_mul(8));
                self.general_regs[i as usize] = self.read_octa(addr);
            }
        }

        // The hole $x gets the last output; the rest,
        // $(x+1)..$(x+count-1), get the callee's remaining outputs in order.
        let hole = if count == 0 || clamped {
            0
        } else {
            returns[(count - 1) as usize]
        };
        if (x as usize) < rg as usize {
            self.general_regs[x as usize] = hole;
        }
        for i in 1..count {
            let dst = (x as u16) + i;
            if dst < rg as u16 {
                self.general_regs[dst as usize] = returns[(i - 1) as usize];
            }
        }

        let new_rl = std::cmp::min((x as u16) + count, rg as u16) as u64;

        self.set_special(SpecialReg::RO, new_ro);
        self.set_special(SpecialReg::RS, new_ro);
        self.set_special(SpecialReg::RL, new_rl);

        let return_pc = self.get_special(SpecialReg::RJ);

        PopFrame::Frame(return_pc.wrapping_add((yz as u64) * 4))
    }

    /// Push the machine's context onto the register stack, growing upward
    /// from the current `rO`. From lowest address to highest: the `rL`
    /// local registers `$0..$(rL-1)`, a marker octa holding `rL`, the
    /// global registers `$rG..$255`, the twelve `SAVE_SPECIALS` in order,
    /// and one packed octa with `rG` in its top byte and `rA` in its low
    /// bits. `$X` receives the packed octa's address; `rO` and `rS` both
    /// become the address of the byte after it, and `rL` becomes 0. `rJ` is
    /// saved as data among the specials, never written live: `SAVE` opens
    /// no call frame.
    ///
    /// `X` must already be global (`X >= rG`) — a local destination halts
    /// with the machine unchanged. `writes_general_register_x` would
    /// otherwise pre-claim a local `X` as a side effect of dispatch before
    /// this check ever runs; the preamble in `execute_instruction` skips
    /// that pre-claim for `SAVE` so a rejected `SAVE` truly touches
    /// nothing.
    pub(super) fn save_context(&mut self, x: u8) -> bool {
        let rg = self.get_special(SpecialReg::RG) as u8;
        if x < rg {
            return self.reject(&format!(
                "SAVE $X,0: X={x} must name a global (rG={rg}) at PC={:#018x}",
                self.pc
            ));
        }

        let rl = self.get_special(SpecialReg::RL);
        let ra = self.get_special(SpecialReg::RA);
        let ro = self.get_special(SpecialReg::RO);

        for i in 0..rl {
            let addr = ro.wrapping_add(i.wrapping_mul(8));
            self.write_octa(addr, self.general_regs[i as usize]);
        }
        let marker_addr = ro.wrapping_add(rl.wrapping_mul(8));
        self.write_octa(marker_addr, rl);

        let globals_base = marker_addr.wrapping_add(8);
        let global_count = 256 - rg as u64;
        for i in 0..global_count {
            let addr = globals_base.wrapping_add(i.wrapping_mul(8));
            self.write_octa(addr, self.general_regs[(rg as u64 + i) as usize]);
        }

        let specials_base = globals_base.wrapping_add(global_count.wrapping_mul(8));
        for (i, reg) in SAVE_SPECIALS.iter().enumerate() {
            let addr = specials_base.wrapping_add((i as u64).wrapping_mul(8));
            self.write_octa(addr, self.get_special(*reg));
        }

        let packed_addr = specials_base.wrapping_add((SAVE_SPECIALS.len() as u64).wrapping_mul(8));
        self.write_octa(packed_addr, (rg as u64) << 56 | ra);

        for reg in 0..rg {
            self.general_regs[reg as usize] = 0;
        }
        self.set_register(x, packed_addr);
        self.set_special(SpecialReg::RL, 0);
        let new_ro = packed_addr.wrapping_add(8);
        self.set_special(SpecialReg::RO, new_ro);
        self.set_special(SpecialReg::RS, new_ro);
        true
    }

    /// Restore the context whose topmost (packed) octa `packed_addr`
    /// addresses, in the layout [`MMix::save_context`] wrote. Validated
    /// whole before any register or memory change: a packed `rG` outside
    /// `32..=255`, a packed `rA` above `RA_MAX`, or a saved local count
    /// above the packed `rG` all halt with the machine unchanged.
    /// Afterward `rL` is the saved local count and `rO = rS` = the address
    /// of the first restored local — where `rO` stood before the matching
    /// `SAVE`.
    pub(super) fn unsave_context(&mut self, packed_addr: u64) -> bool {
        let packed = self.read_octa(packed_addr);
        let rg_saved = (packed >> 56) as u8;
        let ra_saved = packed & !(0xFFu64 << 56);

        if !(32..=255).contains(&rg_saved) {
            return self.reject(&format!(
                "UNSAVE 0,$Z: saved rG={rg_saved} outside 32..=255 at PC={:#018x}",
                self.pc
            ));
        }
        if ra_saved > RA_MAX {
            return self.reject(&format!(
                "UNSAVE 0,$Z: saved rA={ra_saved:#x} exceeds {RA_MAX:#x} at PC={:#018x}",
                self.pc
            ));
        }

        let layout = match self.save_context_layout(packed_addr, rg_saved) {
            Ok(layout) => layout,
            Err(local_count) => {
                return self.reject(&format!(
                    "UNSAVE 0,$Z: saved local count {local_count} exceeds saved \
                     rG={rg_saved} at PC={:#018x}",
                    self.pc
                ));
            }
        };

        for reg in 0..rg_saved {
            self.general_regs[reg as usize] = 0;
        }
        for i in 0..layout.local_count {
            let addr = layout.locals_base.wrapping_add(i.wrapping_mul(8));
            self.general_regs[i as usize] = self.read_octa(addr);
        }
        for i in 0..layout.global_count {
            let addr = layout.globals_base.wrapping_add(i.wrapping_mul(8));
            self.general_regs[(rg_saved as u64 + i) as usize] = self.read_octa(addr);
        }
        for (i, reg) in SAVE_SPECIALS.iter().enumerate() {
            let addr = layout
                .specials_base
                .wrapping_add((i as u64).wrapping_mul(8));
            let value = self.read_octa(addr);
            self.set_special(*reg, value);
        }

        self.set_special(SpecialReg::RG, rg_saved as u64);
        self.set_special(SpecialReg::RA, ra_saved);
        self.set_special(SpecialReg::RL, layout.local_count);
        self.set_special(SpecialReg::RO, layout.locals_base);
        self.set_special(SpecialReg::RS, layout.locals_base);
        true
    }

    /// The arithmetic below a saved context's packed octa at `packed_addr`,
    /// given its already-extracted `rg_saved`: specials, globals, the
    /// marker and the locals, in the layout [`MMix::save_context`] wrote.
    /// `Err` carries the saved local count when it exceeds `rg_saved` —
    /// corrupt or forged memory, never something `SAVE` itself would
    /// write. Both [`MMix::unsave_context`] and [`MMix::call_depth`] rely
    /// on this bound: it is what keeps the latter's walk from stepping
    /// somewhere memory content, not address arithmetic, decided.
    fn save_context_layout(
        &self,
        packed_addr: u64,
        rg_saved: u8,
    ) -> Result<SaveContextLayout, u64> {
        let specials_base = packed_addr.wrapping_sub((SAVE_SPECIALS.len() as u64).wrapping_mul(8));
        let global_count = 256 - rg_saved as u64;
        let globals_base = specials_base.wrapping_sub(global_count.wrapping_mul(8));
        let marker_addr = globals_base.wrapping_sub(8);
        let local_count = self.read_octa(marker_addr);
        if local_count > rg_saved as u64 {
            return Err(local_count);
        }
        let locals_base = marker_addr.wrapping_sub(local_count.wrapping_mul(8));
        Ok(SaveContextLayout {
            specials_base,
            globals_base,
            global_count,
            local_count,
            locals_base,
        })
    }

    /// Current call depth: the number of PUSHJ/PUSHGO frames not yet
    /// popped. Walks the register stack down from `rO` to
    /// [`STACK_SEGMENT_START`]: a marker octa below 256 closes out a
    /// frame and counts; a `SAVE` context — its topmost octa never below
    /// 256 — is skipped whole and does not count. Stops, returning the
    /// count so far, if `rO` is ever misaligned or outside the stack
    /// segment, or if a step's target does not lie strictly below where it
    /// started — the walk trusts address arithmetic, never a value read
    /// from memory, to bound itself, so no stack content can make it loop.
    pub fn call_depth(&self) -> usize {
        let mut ro = self.get_special(SpecialReg::RO);
        let mut depth = 0;
        while ro > STACK_SEGMENT_START && ro < 0x8000000000000000 && ro.is_multiple_of(8) {
            let top = self.read_octa(ro.wrapping_sub(8));
            let next = if top < 256 {
                depth += 1;
                ro.wrapping_sub((top + 1).wrapping_mul(8))
            } else {
                let rg_saved = (top >> 56) as u8;
                match self.save_context_layout(ro.wrapping_sub(8), rg_saved) {
                    Ok(layout) => layout.locals_base,
                    Err(_) => break,
                }
            };
            if next >= ro {
                break;
            }
            ro = next;
        }
        depth
    }
}
