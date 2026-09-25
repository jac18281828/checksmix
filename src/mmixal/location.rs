//! The location counter: address validity, alignment, and instruction sizing.

use super::MMixAssembler;
use super::Rule;
use super::instructions::MMixInstruction;

impl MMixAssembler {
    /// Peek at instruction type to determine size without modifying state
    pub(super) fn peek_instruction_type(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<MMixInstruction, String> {
        let inner = pair.into_inner().next().ok_or("Empty instruction")?;

        match inner.as_rule() {
            // SET is one tetra whichever variant it selects.
            Rule::inst_set => Ok(MMixInstruction::SETRR(0, 0)),
            Rule::inst_seti => Ok(MMixInstruction::SET(0, 0)),
            Rule::inst_setl_ri => Ok(MMixInstruction::SETL(0, 0)),
            Rule::inst_seth_ri => Ok(MMixInstruction::SETH(0, 0)),
            Rule::inst_setmh_ri => Ok(MMixInstruction::SETMH(0, 0)),
            Rule::inst_setml_ri => Ok(MMixInstruction::SETML(0, 0)),
            Rule::inst_incl_ri => Ok(MMixInstruction::INCL(0, 0)),
            // LDA $X,$Y,Z (3-operand form) always emits a real 4-byte LDA
            // regardless of Z's value. The 2-operand form resolves against
            // a GREG base like the memory forms and is the same one tetra
            // always, so it falls to the catch-all.
            Rule::inst_lda_rri => Ok(MMixInstruction::LDA(0, 0, 0)),
            Rule::inst_halt => Ok(MMixInstruction::HALT),
            // For all other instructions, return a standard 4-byte instruction
            _ => Ok(MMixInstruction::ADDU(0, 0, 0)),
        }
    }

    /// `Err` once the counter has passed the end, for a site that names no
    /// item of its own -- a standalone label, a `LOC` label, a `BSPEC`
    /// label or the `@` symbol.
    fn valid_addr(&self) -> Result<(), ()> {
        if self.past_end { Err(()) } else { Ok(()) }
    }

    /// [`Self::valid_addr`] at `site`, as the located error
    /// [`Self::require_addr`] reports.
    pub(super) fn require_valid(&self, site: (usize, usize)) -> Result<(), String> {
        Self::require_addr(self.valid_addr(), &self.current_filename, site)
    }

    /// Round the location counter up to `alignment`, the way MMIXAL does
    /// before it assembles an item: a label on that line names the rounded
    /// address, and the skipped bytes are a gap rather than emitted padding.
    /// `Err` when the counter has already passed the end, or rounding up
    /// would need an address past it -- `current_addr` is left unchanged in
    /// that case.
    ///
    /// Both passes round at the same point, ahead of the item's operands, so
    /// a forward reference sees the same address in either pass.
    pub(super) fn align_current_addr(&mut self, alignment: u64) -> Result<(), ()> {
        if self.past_end {
            return Err(());
        }
        match self.current_addr.checked_next_multiple_of(alignment) {
            Some(addr) => {
                self.current_addr = addr;
                Ok(())
            }
            None => {
                self.past_end = true;
                Err(())
            }
        }
    }

    /// Reserves `size` bytes at the current address for one item, advancing
    /// the counter past them. `Err` when the counter is already past the
    /// end, or when this item's own bytes would need one past
    /// `#FFFFFFFFFFFFFFFF` -- `current_addr` is left unchanged either way.
    /// `Ok` even when the item fills the address space's last byte exactly,
    /// which only marks the counter past the end for whatever comes next.
    pub(super) fn place_item(&mut self, size: u64) -> Result<(), ()> {
        if self.past_end {
            return Err(());
        }
        let end = u128::from(self.current_addr) + u128::from(size);
        let past_last_addr = u128::from(u64::MAX) + 1;
        if end > past_last_addr {
            return Err(());
        }
        if end == past_last_addr {
            self.past_end = true;
        } else {
            self.current_addr = end as u64;
        }
        Ok(())
    }

    /// `Ok` when the address `align_current_addr`/`place_item`/`valid_addr`
    /// reported passes through unchanged; `Err` naming `site` -- a pending
    /// label's or local label's own site when one sits ahead of the item
    /// that failed, the item's own site otherwise.
    pub(super) fn require_addr(
        result: Result<(), ()>,
        filename: &str,
        site: (usize, usize),
    ) -> Result<(), String> {
        result.map_err(|()| {
            let (line, col) = site;
            format!("{filename}:{line}:{col}: address past #FFFFFFFFFFFFFFFF")
        })
    }

    /// Every instruction occupies a tetra-aligned slot, including the
    /// pseudo-instructions `instruction_size` reports as wider than one tetra.
    /// Alignment is not the emitted size.
    pub(super) const INSTRUCTION_ALIGNMENT: u64 = 4;

    pub(super) fn instruction_size(inst: &MMixInstruction) -> u64 {
        match inst {
            MMixInstruction::SET(_, _) => 16,
            MMixInstruction::SETRR(_, _) => 4, // ORI $X, $Y, 0
            MMixInstruction::BYTE(_) => 1,
            MMixInstruction::WYDE(_) => 2,
            MMixInstruction::TETRA(_) => 4,
            MMixInstruction::OCTA(_) => 8,
            _ => 4,
        }
    }
}
