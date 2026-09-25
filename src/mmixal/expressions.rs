//! Expression evaluation: operator precedence, data-list folding, and literal decoding.

use super::MMixAssembler;
use super::Rule;
use super::SymbolType;

/// What an MMIXAL expression evaluates to: a pure 64-bit value, or a register
/// number. `Register` carries the full 64-bit value unary `$` produced, or
/// register arithmetic derived from one -- range-checking against 0..=255
/// happens where a register value is finally consumed, not here, since an
/// intermediate register-typed value may exceed 255 mid-expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ExprValue {
    Pure(u64),
    Register(u64),
}

/// `fold_data_atoms`'s `child` parameter: evaluates one `data_term` or
/// `data_primary` into the atoms it contributes.
type EvalOperand = fn(&MMixAssembler, pest::iterators::Pair<Rule>) -> Result<DataAtoms, String>;

/// `fold_data_atoms`'s `apply` parameter, applying a binary operator to two
/// atoms. `apply_weak` and `apply_strong` share this signature.
type ApplyOperator =
    fn(&MMixAssembler, &str, ExprValue, ExprValue, usize, usize) -> Result<ExprValue, String>;

/// One or more values a `data_primary`, `data_term` or `data_expr`
/// contributes: a string's every character is its own atom, so a
/// multi-character string's interior stays free of whatever touches its
/// neighbors while its first and last atom combine with them. Emptiness is
/// unrepresentable, so combining two lists across an operator needs no
/// guard for it.
struct DataAtoms {
    head: ExprValue,
    tail: Vec<ExprValue>,
}

impl DataAtoms {
    fn one(value: ExprValue) -> Self {
        DataAtoms {
            head: value,
            tail: Vec::new(),
        }
    }

    fn from_chars(first: u32, rest: Vec<u32>) -> Self {
        DataAtoms {
            head: ExprValue::Pure(first as u64),
            tail: rest
                .into_iter()
                .map(|ch| ExprValue::Pure(ch as u64))
                .collect(),
        }
    }

    fn last(&self) -> ExprValue {
        *self.tail.last().unwrap_or(&self.head)
    }

    fn last_mut(&mut self) -> &mut ExprValue {
        self.tail.last_mut().unwrap_or(&mut self.head)
    }

    fn into_vec(self) -> Vec<ExprValue> {
        let mut items = vec![self.head];
        items.extend(self.tail);
        items
    }

    /// Merges `self` and `other` across an operator: combines `self`'s last
    /// atom with `other`'s first via `combine`, leaving every other atom in
    /// place.
    fn merge(
        mut self,
        other: DataAtoms,
        combine: impl FnOnce(ExprValue, ExprValue) -> Result<ExprValue, String>,
    ) -> Result<DataAtoms, String> {
        let combined = combine(self.last(), other.head)?;
        *self.last_mut() = combined;
        self.tail.extend(other.tail);
        Ok(self)
    }
}

impl MMixAssembler {
    /// Decode a data directive's string literal content into the values it
    /// represents: one item per character, its Unicode scalar value. Both
    /// passes go through this single function so neither can disagree with
    /// the other on a string's size.
    fn decode_char_values(content: &str) -> Vec<u32> {
        content.chars().map(|ch| ch as u32).collect()
    }

    /// Evaluate `pair` (an `expr`, or one of the rules it nests) under the
    /// register/pure-value rules in `MMIX.md`'s Expressions section: `+ - *`
    /// wrap mod 2^64, `/` `//` `%` `<<` `>>` follow the reference's
    /// definitions, `@` is the current location, and a symbol carries
    /// whichever kind its `SymbolType` records. Register arithmetic combines
    /// a register with a pure value into a register (register-pure
    /// subtraction included); register-register subtraction gives a pure
    /// value; every other operator applied to a register is an error, as is
    /// every non-`+` unary operator.
    pub(super) fn eval_expr(&self, pair: pest::iterators::Pair<Rule>) -> Result<ExprValue, String> {
        match pair.as_rule() {
            // Wraps exactly one `expr`; some call sites hand this container
            // pair straight to the evaluator unwrapped.
            Rule::operand_list_one => self.eval_expr(
                pair.into_inner()
                    .next()
                    .expect("operand wraps exactly one expr"),
            ),
            Rule::expr | Rule::group_expr | Rule::data_group_expr => {
                let mut parts = pair.into_inner();
                let mut acc = self.eval_expr(parts.next().expect("expr has a term"))?;
                while let Some(op) = parts.next() {
                    let (line, col) = op.line_col();
                    let rhs = self.eval_expr(parts.next().expect("weak operator needs a term"))?;
                    acc = self.apply_weak(op.as_str(), acc, rhs, line, col)?;
                }
                Ok(acc)
            }
            Rule::term | Rule::group_term | Rule::data_group_term => {
                let mut parts = pair.into_inner();
                let mut acc = self.eval_expr(parts.next().expect("term has a primary"))?;
                while let Some(op) = parts.next() {
                    let (line, col) = op.line_col();
                    let rhs =
                        self.eval_expr(parts.next().expect("strong operator needs a primary"))?;
                    acc = self.apply_strong(op.as_str(), acc, rhs, line, col)?;
                }
                Ok(acc)
            }
            Rule::primary | Rule::group_primary | Rule::data_group_primary => {
                let (line, col) = pair.line_col();
                let mut parts = pair.into_inner();
                let first = parts.next().expect("primary has a child");
                if first.as_rule() == Rule::unary_op {
                    let operand =
                        self.eval_expr(parts.next().expect("unary operator needs an operand"))?;
                    self.apply_unary(first.as_str(), operand, line, col)
                } else {
                    self.eval_expr(first)
                }
            }
            Rule::group | Rule::data_group => self.eval_expr(
                pair.into_inner()
                    .next()
                    .expect("group has an inner expression"),
            ),
            // Reached only through `data_group_primary`: a parenthesized
            // group always needs a single value.
            Rule::string_literal => self.eval_group_string(pair),
            Rule::at_symbol => {
                let (line, col) = pair.line_col();
                self.require_valid((line, col))?;
                Ok(ExprValue::Pure(self.current_addr))
            }
            Rule::constant => self.eval_literal(
                pair.into_inner()
                    .next()
                    .expect("constant has exactly one literal"),
            ),
            Rule::global_id => {
                let (line, col) = pair.line_col();
                let text = pair.as_str();
                let qualified = self.qualify_name(text);
                // Labels before symbols: a program's own label wins over a
                // predefined symbol of the same name (a user IS/GREG
                // symbol already overwrites the predefined entry directly
                // in `symbols`, so this order alone is what a label needs).
                if let Some(&label_addr) = self.labels.get(&qualified) {
                    Ok(ExprValue::Pure(label_addr))
                } else if let Some(&symbol_type) = self.symbols.get(&qualified) {
                    Ok(match symbol_type {
                        SymbolType::Constant(value) => ExprValue::Pure(value),
                        SymbolType::Register(reg) => ExprValue::Register(reg as u64),
                    })
                } else {
                    Err(format!(
                        "{}:{}:{}: Undefined symbol: {}",
                        self.current_filename, line, col, qualified
                    ))
                }
            }
            Rule::local_ref_back => {
                let digit = Self::local_digit(pair.as_str());
                Ok(self.resolve_local_back(digit))
            }
            Rule::local_ref_fwd => {
                let (line, col) = pair.line_col();
                let digit = Self::local_digit(pair.as_str());
                self.resolve_local_fwd(digit, line, col)
            }
            other => {
                let (line, col) = pair.line_col();
                Err(format!(
                    "{}:{}:{}: Expected expression, got: {:?}",
                    self.current_filename, line, col, other
                ))
            }
        }
    }

    /// A `string_literal` standing where a parenthesized group needs its
    /// one value: its single character, or an error naming how many
    /// characters it holds. `eval_data_primary_atoms` is the path for a
    /// string outside a group, where it may expand to more than one item.
    fn eval_group_string(&self, pair: pest::iterators::Pair<Rule>) -> Result<ExprValue, String> {
        let (line, col) = pair.line_col();
        let (first, rest) = self.decode_data_string_literal(&pair)?;
        if rest.is_empty() {
            return Ok(ExprValue::Pure(first as u64));
        }
        Err(format!(
            "{}:{}:{}: a {}-character string is not a single value here",
            self.current_filename,
            line,
            col,
            1 + rest.len()
        ))
    }

    /// Whether `data_expr` is `BYTE ""`'s one exempt shape: a bare string,
    /// with no unary wrap, sibling term or primary, and no content. Its
    /// empty string assembles as one zero unit and warns, not the error
    /// every other position gives one.
    pub(super) fn is_bare_empty_string(data_expr: &pest::iterators::Pair<Rule>) -> bool {
        let mut terms = data_expr.clone().into_inner();
        let Some(term) = terms.next() else {
            return false;
        };
        if terms.next().is_some() {
            return false;
        }
        let mut primaries = term.into_inner();
        let Some(primary) = primaries.next() else {
            return false;
        };
        if primaries.next().is_some() {
            return false;
        }
        let mut children = primary.into_inner();
        let Some(child) = children.next() else {
            return false;
        };
        if children.next().is_some() || child.as_rule() != Rule::string_literal {
            return false;
        }
        let text = child.as_str();
        text.len() == 2
    }

    /// A `data_value`'s inner `data_expr`, present whenever the grammar
    /// built the node.
    pub(super) fn data_expr(
        value: pest::iterators::Pair<Rule>,
    ) -> Result<pest::iterators::Pair<Rule>, String> {
        value
            .into_inner()
            .next()
            .ok_or_else(|| "Missing data expression".to_string())
    }

    /// A `string_literal`'s content, decoded to its first character's
    /// Unicode scalar value and the rest. Rejects an empty string with an
    /// error at its own position; the one exemption, `BYTE ""` standing
    /// entirely alone, is caught by [`Self::is_bare_empty_string`] before
    /// either pass reaches this.
    pub(super) fn decode_data_string_literal(
        &self,
        string_pair: &pest::iterators::Pair<Rule>,
    ) -> Result<(u32, Vec<u32>), String> {
        let (line, col) = string_pair.line_col();
        let text = string_pair.as_str();
        let mut values = Self::decode_char_values(&text[1..text.len() - 1]).into_iter();
        let first = values.next().ok_or_else(|| {
            format!(
                "{}:{}:{}: an empty string is not a value inside an expression",
                self.current_filename, line, col
            )
        })?;
        Ok((first, values.collect()))
    }

    /// Evaluate a `data_value` (a `data_expr`) into the values it
    /// contributes to its directive's list. A string stands wherever a
    /// `data_primary` stands: an operator directly on it combines with its
    /// first character (a leading unary or the operator before it) or its
    /// last (the operator after it), and each character between stays its
    /// own item. A string standing alone, with no operator anywhere,
    /// contributes one item per character. A bare `""`, the value's only
    /// content, contributes one zero item; the caller warns.
    pub(super) fn eval_data_value_items(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<Vec<ExprValue>, String> {
        let data_expr = Self::data_expr(pair)?;
        if Self::is_bare_empty_string(&data_expr) {
            return Ok(vec![ExprValue::Pure(0)]);
        }
        Ok(self.eval_data_expr_atoms(data_expr)?.into_vec())
    }

    /// Folds a `data_expr`'s `data_term`s or a `data_term`'s
    /// `data_primary`s into one [`DataAtoms`]: `child` evaluates each
    /// operand and `apply` combines a pair of atoms across the operator
    /// between them, via [`DataAtoms::merge`].
    fn fold_data_atoms(
        &self,
        pair: pest::iterators::Pair<Rule>,
        child: EvalOperand,
        apply: ApplyOperator,
    ) -> Result<DataAtoms, String> {
        let mut parts = pair.into_inner();
        let mut atoms = child(
            self,
            parts
                .next()
                .ok_or_else(|| "a fold operand list is never empty".to_string())?,
        )?;
        while let Some(op) = parts.next() {
            let (line, col) = op.line_col();
            let rhs = child(
                self,
                parts
                    .next()
                    .ok_or_else(|| "an operator needs a right operand".to_string())?,
            )?;
            atoms = atoms.merge(rhs, |lhs, rhs| {
                apply(self, op.as_str(), lhs, rhs, line, col)
            })?;
        }
        Ok(atoms)
    }

    /// [`Self::eval_data_value_items`]'s fold across a `data_expr`'s weak
    /// operators: each `data_term` contributes one or more atoms, and a
    /// weak operator combines the last atom before it with the first atom
    /// after -- every atom between stays its own item.
    fn eval_data_expr_atoms(&self, pair: pest::iterators::Pair<Rule>) -> Result<DataAtoms, String> {
        self.fold_data_atoms(pair, Self::eval_data_term_atoms, Self::apply_weak)
    }

    /// [`Self::eval_data_expr_atoms`]'s counterpart for a `data_term`'s
    /// strong operators, combining `data_primary` atom lists the same way.
    fn eval_data_term_atoms(&self, pair: pest::iterators::Pair<Rule>) -> Result<DataAtoms, String> {
        self.fold_data_atoms(pair, Self::eval_data_primary_atoms, Self::apply_strong)
    }

    /// A `data_primary`'s atoms: one, for an ordinary value, or one per
    /// character for a string. A unary operator combines with the first
    /// atom of its operand only -- `-"ab"` is `-'a'`, `'b'`, matching
    /// [`DataAtoms::merge`]'s treatment of a binary operator's neighbor.
    fn eval_data_primary_atoms(
        &self,
        pair: pest::iterators::Pair<Rule>,
    ) -> Result<DataAtoms, String> {
        let (line, col) = pair.line_col();
        let mut parts = pair.into_inner();
        let first = parts
            .next()
            .ok_or_else(|| "data_primary has a child".to_string())?;
        match first.as_rule() {
            Rule::unary_op => {
                let operand = parts
                    .next()
                    .ok_or_else(|| "unary operator needs an operand".to_string())?;
                let mut atoms = self.eval_data_primary_atoms(operand)?;
                atoms.head = self.apply_unary(first.as_str(), atoms.head, line, col)?;
                Ok(atoms)
            }
            Rule::string_literal => {
                let (first_char, rest) = self.decode_data_string_literal(&first)?;
                Ok(DataAtoms::from_chars(first_char, rest))
            }
            _ => Ok(DataAtoms::one(self.eval_expr(first)?)),
        }
    }

    /// Evaluate a leaf numeric literal: hex, decimal or char.
    fn eval_literal(&self, pair: pest::iterators::Pair<Rule>) -> Result<ExprValue, String> {
        let (line, col) = pair.line_col();
        let text = pair.as_str();

        let value = match pair.as_rule() {
            Rule::char_literal => {
                let inner = &text[1..text.len() - 1];
                let ch = inner.chars().next().ok_or_else(|| {
                    "grammar admits exactly one character between the quotes".to_string()
                })?;
                ch as u32 as u64
            }
            // A digit string the grammar matched always has a value: a hex
            // or decimal constant of 2^64 or more reduces mod 2^64, the
            // reference's own rule, computed digit by digit so no width
            // limit on the source spelling is ever reached.
            Rule::hex_literal => {
                let hex_str = if let Some(stripped) = text.strip_prefix('#') {
                    stripped
                } else if let Some(stripped) =
                    text.strip_prefix("0x").or_else(|| text.strip_prefix("0X"))
                {
                    stripped
                } else {
                    text
                };
                hex_str.chars().try_fold(0u64, |acc, c| {
                    let digit = c
                        .to_digit(16)
                        .ok_or_else(|| "grammar admits only hex digits".to_string())?;
                    Ok::<u64, String>(acc.wrapping_shl(4).wrapping_add(digit as u64))
                })?
            }
            Rule::dec_literal => text.chars().try_fold(0u64, |acc, c| {
                let digit = c
                    .to_digit(10)
                    .ok_or_else(|| "grammar admits only decimal digits".to_string())?;
                Ok::<u64, String>(acc.wrapping_mul(10).wrapping_add(digit as u64))
            })?,
            other => {
                return Err(format!(
                    "{}:{}:{}: Expected a literal, got: {:?}",
                    self.current_filename, line, col, other
                ));
            }
        };
        Ok(ExprValue::Pure(value))
    }

    /// Unary operators: `+` is the identity, including on a register; `-`
    /// and `~` apply only to a pure value; `$` casts a pure value to a
    /// register; `&` (a symbol's serial number) is always rejected, since
    /// checksmix's object file carries no symbol table to index.
    fn apply_unary(
        &self,
        op: &str,
        value: ExprValue,
        line: usize,
        col: usize,
    ) -> Result<ExprValue, String> {
        if op == "&" {
            return Err(format!(
                "{}:{}:{}: unary & (a symbol's serial number) is unsupported",
                self.current_filename, line, col
            ));
        }
        match (op, value) {
            ("+", v) => Ok(v),
            ("-", ExprValue::Pure(v)) => Ok(ExprValue::Pure(0u64.wrapping_sub(v))),
            ("~", ExprValue::Pure(v)) => Ok(ExprValue::Pure(!v)),
            ("$", ExprValue::Pure(v)) => Ok(ExprValue::Register(v)),
            (op, ExprValue::Register(_)) => Err(format!(
                "{}:{}:{}: unary {} cannot apply to a register",
                self.current_filename, line, col, op
            )),
            _ => unreachable!("grammar admits only + - ~ $ & as unary_op"),
        }
    }

    /// Weak (lowest-precedence) binary operators: `+` `-` `|` `^`.
    /// Register arithmetic: register+pure, pure+register and register-pure
    /// give a register; register-register gives a pure value; `|` and `^`
    /// never take a register operand.
    fn apply_weak(
        &self,
        op: &str,
        lhs: ExprValue,
        rhs: ExprValue,
        line: usize,
        col: usize,
    ) -> Result<ExprValue, String> {
        use ExprValue::{Pure, Register};
        match (op, lhs, rhs) {
            ("+", Pure(a), Pure(b)) => Ok(Pure(a.wrapping_add(b))),
            ("-", Pure(a), Pure(b)) => Ok(Pure(a.wrapping_sub(b))),
            ("|", Pure(a), Pure(b)) => Ok(Pure(a | b)),
            ("^", Pure(a), Pure(b)) => Ok(Pure(a ^ b)),
            ("+", Register(a), Pure(b)) | ("+", Pure(b), Register(a)) => {
                Ok(Register(a.wrapping_add(b)))
            }
            ("-", Register(a), Pure(b)) => Ok(Register(a.wrapping_sub(b))),
            ("-", Register(a), Register(b)) => Ok(Pure(a.wrapping_sub(b))),
            _ => Err(format!(
                "{}:{}:{}: {} cannot apply to a register operand",
                self.current_filename, line, col, op
            )),
        }
    }

    /// Strong (highest-precedence) binary operators: `*` `/` `//` `%` `<<`
    /// `>>` `&`. None takes a register operand. `x/y` is illegal at `y=0`;
    /// `x//y` is illegal at `x>=y` (which subsumes `y=0`, since every `x` is
    /// `>=0`); `x%y` shares `x/y`'s zero-divisor rule, computing the
    /// remainder of the same division. A shift of 64 or more gives `0`.
    fn apply_strong(
        &self,
        op: &str,
        lhs: ExprValue,
        rhs: ExprValue,
        line: usize,
        col: usize,
    ) -> Result<ExprValue, String> {
        let (a, b) = match (lhs, rhs) {
            (ExprValue::Pure(a), ExprValue::Pure(b)) => (a, b),
            _ => {
                return Err(format!(
                    "{}:{}:{}: {} cannot apply to a register operand",
                    self.current_filename, line, col, op
                ));
            }
        };
        let value = match op {
            "*" => a.wrapping_mul(b),
            "/" => {
                if b == 0 {
                    return Err(format!(
                        "{}:{}:{}: division by zero in {}/{}",
                        self.current_filename, line, col, a, b
                    ));
                }
                a / b
            }
            "%" => {
                if b == 0 {
                    return Err(format!(
                        "{}:{}:{}: division by zero in {}%{}",
                        self.current_filename, line, col, a, b
                    ));
                }
                a % b
            }
            "//" => {
                if a >= b {
                    return Err(format!(
                        "{}:{}:{}: illegal fraction {}//{} (the dividend must be less than the divisor)",
                        self.current_filename, line, col, a, b
                    ));
                }
                (((a as u128) << 64) / (b as u128)) as u64
            }
            "<<" => {
                if b >= 64 {
                    0
                } else {
                    a << b
                }
            }
            ">>" => {
                if b >= 64 {
                    0
                } else {
                    a >> b
                }
            }
            "&" => a & b,
            _ => unreachable!("grammar admits only * / // % << >> & as strong_op"),
        };
        Ok(ExprValue::Pure(value))
    }
}
