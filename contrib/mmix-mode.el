;;; mmix-mode.el --- Major mode for checksmix MMIXAL source -*- lexical-binding: t; -*-

;; Package-Requires: ((emacs "29.1"))

;;; Commentary:

;; Syntax highlighting, indentation and instruction help for `.mms' files,
;; following the MMIXAL dialect checksmix assembles rather than canonical
;; MMIXAL. Where the two differ this mode follows checksmix:
;;
;;   - `%' starts a comment that runs to the end of the line; `;'
;;     separates statements.
;;   - A string literal has no escapes; a character literal accepts
;;     \n \r \t \0 \\ and \'.
;;   - An indented line has no label field: its first word is the
;;     operation.  In column 1, or after a `;', the first word is a label
;;     unless it is a mnemonic or directive, and a label may carry a
;;     leading `:' (global), interior `:'s (`Foo:Bar', qualified by
;;     PREFIX) and a trailing `:'.  A decimal digit followed by `H'
;;     is a local label instead; `B'/`F' reference the nearest one
;;     behind/ahead of it in an operand.
;;   - Mnemonics and directives match in upper case only; predefined
;;     symbols (`rJ', `StdOut', `Fputs', `ROUND_NEAR', ...) are
;;     case-sensitive in their own spelling, `debug' is checksmix's own
;;     lower-case directive.
;;   - The explicit immediate spellings (ADDI, SETI, GETAB, ...), the
;;     extension HALT, INCLUDE and the `debug "text"' preprocessor line
;;     are all keywords.
;;
;; A name the buffer defines is highlighted where it is used, in the face
;; of its definition: a label as a function name, an IS or GREG name as a
;; variable.  Names match exactly, so `:Foo' and `Foo' are different
;; symbols.  Names from an INCLUDEd file, or qualified by PREFIX, are not
;; tracked.
;;
;; Instruction help is built in.  `eldoc' shows the syntax and description
;; of the instruction on the current line, or of the predefined symbol at
;; point; C-c C-d describes any instruction in a help buffer.
;;
;; C-c C-c runs the buffer's file through `checksmix run' under `compile'.
;;
;; To install, put this file in a directory on `load-path' and require it:
;;
;;   (require 'mmix-mode)

;;; Code:

(require 'cl-lib)
(require 'compile)
(require 'eldoc)
(require 'help-mode)

(defgroup mmix nil
  "Editing checksmix MMIXAL source."
  :group 'languages)

(defcustom mmix-checksmix-program "checksmix"
  "The checksmix executable `mmix-run' invokes."
  :type 'string)

(defcustom mmix-operation-column 8
  "Column at which `mmix-indent-line' places a statement's operation."
  :type 'natnum)

(defcustom mmix-operand-column 16
  "Column at which `mmix-indent-line' places a statement's operands."
  :type 'natnum)

(defconst mmix-instructions
  '("16ADDU" "16ADDUI" "2ADDU" "2ADDUI" "4ADDU" "4ADDUI" "8ADDU" "8ADDUI"
    "ADD" "ADDI" "ADDU" "ADDUI" "AND" "ANDI" "ANDN" "ANDNH" "ANDNI" "ANDNL"
    "ANDNMH" "ANDNML" "BDIF" "BDIFI" "BEV" "BEVB" "BN" "BNB" "BNN" "BNNB"
    "BNP" "BNPB" "BNZ" "BNZB" "BOD" "BODB" "BP" "BPB" "BZ" "BZB"
    "CMP" "CMPI" "CMPU" "CMPUI" "CSEV" "CSEVI" "CSN" "CSNI" "CSNN" "CSNNI"
    "CSNP" "CSNPI" "CSNZ" "CSNZI" "CSOD" "CSODI" "CSP" "CSPI" "CSWAP"
    "CSWAPI" "CSZ" "CSZI" "DIV" "DIVI" "DIVU" "DIVUI" "FADD" "FCMP" "FCMPE"
    "FDIV" "FEQL" "FEQLE" "FINT" "FIX" "FIXU" "FLOT" "FLOTI" "FLOTU"
    "FLOTUI" "FMUL" "FREM" "FSQRT" "FSUB" "FUN" "FUNE" "GET" "GETA" "GETAB"
    "GO" "GOI" "HALT" "INCH" "INCL" "INCMH" "INCML" "JMP" "JMPB" "LDA"
    "LDAI" "LDB" "LDBI" "LDBU" "LDBUI" "LDHT" "LDHTI"
    "LDO" "LDOI" "LDOU" "LDOUI" "LDSF" "LDSFI" "LDT" "LDTI" "LDTU" "LDTUI"
    "LDUNC" "LDUNCI" "LDVTS" "LDVTSI" "LDW" "LDWI" "LDWU" "LDWUI" "MOR"
    "MORI" "MUL" "MULI" "MULU" "MULUI" "MUX" "MUXI" "MXOR" "MXORI" "NAND"
    "NANDI" "NEG" "NEGI" "NEGU" "NEGUI" "NOR" "NORI" "NXOR" "NXORI" "ODIF"
    "ODIFI" "OR" "ORH" "ORI" "ORL" "ORMH" "ORML" "ORN" "ORNI" "PBEV" "PBEVB"
    "PBN" "PBNB" "PBNN" "PBNNB" "PBNP" "PBNPB" "PBNZ" "PBNZB" "PBOD" "PBODB"
    "PBP" "PBPB" "PBZ" "PBZB" "POP" "PREGO" "PREGOI" "PRELD" "PRELDI"
    "PREST" "PRESTI" "PUSHGO" "PUSHGOI" "PUSHJ" "PUSHJB" "PUT" "PUTI"
    "RESUME" "SADD" "SADDI" "SAVE" "SET" "SETH" "SETI" "SETL" "SETMH"
    "SETML" "SFLOT" "SFLOTI" "SFLOTU" "SFLOTUI" "SL" "SLI" "SLU" "SLUI" "SR"
    "SRI" "SRU" "SRUI" "STB" "STBI" "STBU" "STBUI" "STCO" "STCOI" "STHT"
    "STHTI" "STO" "STOI" "STOU" "STOUI" "STSF" "STSFI" "STT" "STTI" "STTU"
    "STTUI" "STUNC" "STUNCI" "STW" "STWI" "STWU" "STWUI" "SUB" "SUBI" "SUBU"
    "SUBUI" "SWYM" "SYNC" "SYNCD" "SYNCDI" "SYNCID" "SYNCIDI" "TDIF" "TDIFI"
    "TRAP" "TRIP" "UNSAVE" "WDIF" "WDIFI" "XOR" "XORI" "ZSEV" "ZSEVI" "ZSN"
    "ZSNI" "ZSNN" "ZSNNI" "ZSNP" "ZSNPI" "ZSNZ" "ZSNZI" "ZSOD" "ZSODI" "ZSP"
    "ZSPI" "ZSZ" "ZSZI")
  "Every instruction mnemonic checksmix assembles, in upper case.")

(defconst mmix-directives
  '("BYTE" "WYDE" "TETRA" "OCTA" "LOC" "GREG" "IS" "PREFIX" "INCLUDE"
    "LOCAL" "BSPEC" "ESPEC")
  "Every assembler directive checksmix accepts, in upper case.")

(defconst mmix-debug-directive "debug"
  "checksmix's case-sensitive preprocessor line, `debug \"text\"'.")

(defconst mmix-special-registers
  '(("rB" 0 "bootstrap register (trip)")
    ("rD" 1 "dividend register")
    ("rE" 2 "epsilon register")
    ("rH" 3 "himult register")
    ("rJ" 4 "return-jump register")
    ("rM" 5 "multiplex mask register")
    ("rR" 6 "remainder register")
    ("rBB" 7 "bootstrap register (trap)")
    ("rC" 8 "cycle counter")
    ("rN" 9 "serial number")
    ("rO" 10 "register stack offset")
    ("rS" 11 "register stack pointer")
    ("rI" 12 "interval counter")
    ("rT" 13 "trap address register")
    ("rTT" 14 "dynamic trap address register")
    ("rK" 15 "interrupt mask register")
    ("rQ" 16 "interrupt request register")
    ("rU" 17 "usage counter")
    ("rV" 18 "virtual translation register")
    ("rG" 19 "global threshold register")
    ("rL" 20 "local threshold register")
    ("rA" 21 "arithmetic status register")
    ("rF" 22 "failure location register")
    ("rP" 23 "prediction register")
    ("rW" 24 "where-interrupted register (trip)")
    ("rX" 25 "execution register (trip)")
    ("rY" 26 "Y operand (trip)")
    ("rZ" 27 "Z operand (trip)")
    ("rWW" 28 "where-interrupted register (trap)")
    ("rXX" 29 "execution register (trap)")
    ("rYY" 30 "Y operand (trap)")
    ("rZZ" 31 "Z operand (trap)"))
  "Special register names checksmix predefines: (NAME NUMBER MEANING).")

(defconst mmix-predefined-constants
  '(("Data_Segment" . "start of the data segment, #2000000000000000")
    ("Pool_Segment" . "start of the pool segment, #4000000000000000")
    ("Stack_Segment" . "start of the stack segment, #6000000000000000")
    ("StdIn" . "standard input handle")
    ("StdOut" . "standard output handle")
    ("StdErr" . "standard error handle")
    ("TextRead" . "Fopen mode 0: grants read")
    ("TextWrite" . "Fopen mode 1: grants write")
    ("BinaryRead" . "Fopen mode 2: grants read and seek")
    ("BinaryWrite" . "Fopen mode 3: grants write and seek")
    ("BinaryReadWrite"
     . "Fopen mode 4: grants read, write and seek, switching between read and write")
    ("Halt" . "TRAP function 0: Stop execution; TRAP 0,Halt,Handle; $255 = exit code")
    ("Fopen"
     . "TRAP function 1: Open a file; TRAP 0,Fopen,Handle; name address, mode; returns 0, or −1")
    ("Fclose"
     . "TRAP function 2: Close Handle; TRAP 0,Fclose,Handle; returns 0, or −1")
    ("Fread"
     . "TRAP function 3: Read from Handle; TRAP 0,Fread,Handle; buffer, size; returns 0, n−size at end of file, or −1−size on error")
    ("Fgets"
     . "TRAP function 4: Read a line from Handle; TRAP 0,Fgets,Handle; buffer, size; returns characters stored, or −1")
    ("Fgetws"
     . "TRAP function 5: Read a wide string from Handle; TRAP 0,Fgetws,Handle; buffer, size; returns wydes stored, or −1")
    ("Fwrite"
     . "TRAP function 6: Write to Handle; TRAP 0,Fwrite,Handle; buffer, size; returns 0, or n−size after writing n")
    ("Fputs"
     . "TRAP function 7: Write a null-terminated string to Handle; TRAP 0,Fputs,Handle; $255 = string address; returns bytes written, or −1")
    ("Fputws"
     . "TRAP function 8: Write a null-terminated wide string to Handle; TRAP 0,Fputws,Handle; $255 = string address; returns wydes written, or −1")
    ("Fseek"
     . "TRAP function 9: Seek within Handle; TRAP 0,Fseek,Handle; $255 = offset; returns 0, or −1")
    ("Ftell"
     . "TRAP function 10: Get the current position in Handle; TRAP 0,Ftell,Handle; returns position in $255, or −1")
    ("Fputc"
     . "TRAP function #80: Write one byte to Handle; TRAP 0,Fputc,Handle; $255's low byte; returns 0, or −1")
    ("Time"
     . "TRAP function #81: Report the current time; TRAP 0,Time,Handle; Handle selects seconds (0), milliseconds (1) or microseconds (2); returns the time in $255")
    ("Debug"
     . "TRAP function #82: Backs the debug \"text\" directive; TRAP 0,Debug,K, K the directive's 0-based index")
    ("ROUND_CURRENT" . "rounding-mode override 0: use rA's mode")
    ("ROUND_OFF" . "rounding-mode override 1: toward zero")
    ("ROUND_UP" . "rounding-mode override 2: toward +infinity")
    ("ROUND_DOWN" . "rounding-mode override 3: toward -infinity")
    ("ROUND_NEAR" . "rounding-mode override 4: to nearest, ties to even")
    ("Inf" . "positive floating-point infinity, #7FF0000000000000")
    ("D_BIT" . "rA event-flag bit #80: divide check")
    ("D_Handler" . "user-trip handler address #10, for D_BIT")
    ("V_BIT" . "rA event-flag bit #40: integer overflow")
    ("V_Handler" . "user-trip handler address #20, for V_BIT")
    ("W_BIT" . "rA event-flag bit #20: float-to-fix overflow")
    ("W_Handler" . "user-trip handler address #30, for W_BIT")
    ("I_BIT" . "rA event-flag bit #10: invalid floating operation")
    ("I_Handler" . "user-trip handler address #40, for I_BIT")
    ("O_BIT" . "rA event-flag bit #08: floating overflow")
    ("O_Handler" . "user-trip handler address #50, for O_BIT")
    ("U_BIT" . "rA event-flag bit #04: floating underflow")
    ("U_Handler" . "user-trip handler address #60, for U_BIT")
    ("Z_BIT" . "rA event-flag bit #02: floating division by zero")
    ("Z_Handler" . "user-trip handler address #70, for Z_BIT")
    ("X_BIT" . "rA event-flag bit #01: floating inexact result")
    ("X_Handler" . "user-trip handler address #80, for X_BIT"))
  "Constant symbols checksmix predefines, other than special registers.
Each is (NAME . MEANING).")

(defconst mmix--keywords
  (let ((table (make-hash-table :test #'equal)))
    (dolist (name mmix-instructions) (puthash name 'instruction table))
    (dolist (name mmix-directives) (puthash name 'directive table))
    (puthash mmix-debug-directive 'directive table)
    table)
  "Map from a keyword, spelled as `mmix--keyword-key' returns it, to its kind.")

(defun mmix--keyword-key (word)
  "Return WORD unchanged: the key the keyword tables look WORD up under.
Keywords match in upper case only, except `debug', checksmix's own
lower-case directive, which the tables already hold under that spelling.
This seam stays so every lookup -- fontification, help, eldoc -- shares
one place that decides how a word reaches the tables."
  word)

(defun mmix--keyword-kind (word)
  "Return `instruction', `directive' or nil for WORD."
  (gethash (mmix--keyword-key word) mmix--keywords))

;;;; Statements

(defconst mmix--standalone-instructions
  '("HALT" "SWYM" "ESPEC" "POP" "RESUME" "SYNC" "TRAP" "TRIP")
  "Instructions that form a whole statement with no operands.")

(defconst mmix--single-operand-keywords
  '("JMP" "JMPB" "RESUME" "SYNC" "LOC" "GREG" "PREFIX" "BYTE"
    "WYDE" "TETRA" "OCTA" "LOCAL" "BSPEC" "TRAP" "TRIP" "SWYM" "POP"
    "UNSAVE")
  "Keywords whose statement is complete with a single operand.")

(defconst mmix--rest-of-line-directives '("INCLUDE")
  "Directives whose operand is the rest of the line, whatever it spells.")

(defconst mmix--value-directives '("IS" "GREG")
  "Directives that bind their label to a value rather than an address.")

(defconst mmix--bare-name-directives '("IS")
  "Directives that name a bare symbol, so a label with a trailing colon
before one is a syntax error.")

(defconst mmix--word-regexp
  (rx (? ":") (+ (any alnum "_")) (? ":"))
  "A word that may stand in a statement's label or operation field.")

(defconst mmix--label-regexp
  (rx bos (? ":") (any alpha "_") (* (any alnum "_")) (? ":") eos)
  "A label: an optionally global symbol with an optional trailing colon.")

(cl-defstruct (mmix-statement (:constructor mmix--make-statement)
                              (:copier nil))
  "The label and operation fields of one source line.
A field is its text and the positions it starts and ends at, all nil
when the line lacks it.  OPERANDS-BEG is where the operands start, or
nil when none follow the operation."
  label label-beg label-end
  operation operation-beg operation-end
  operands-beg)

(defun mmix--forward-word ()
  "Move over the statement word at point and the blanks after it.
Return the word's bounds as (BEG . END), or nil, without moving, when
no word is at point."
  (when (looking-at mmix--word-regexp)
    (goto-char (match-end 0))
    (skip-chars-forward " \t")
    (cons (match-beginning 0) (match-end 0))))

(defun mmix--bounds-text (bounds)
  "Return the text between the positions of BOUNDS, (BEG . END), or nil."
  (and bounds (buffer-substring-no-properties (car bounds) (cdr bounds))))

(defun mmix--operands-at-point-p ()
  "Return non-nil when point is before this statement's own text.
Nil at end of line, before a `%' comment, or before a `;' -- checksmix
reads a `;' as the start of a new statement, which this mode does not
model; treating it the same as a comment stops this statement's own
text there."
  (not (or (eolp) (looking-at-p "[%;]"))))

(defun mmix--line-statement ()
  "Return the current line's `mmix-statement', or nil when it has none.
`mmix--operation-first-p' decides whether the first word is a label or
the operation."
  (save-excursion
    (beginning-of-line)
    (unless (nth 3 (syntax-ppss))
      (let ((indented (looking-at-p "[ \t]")))
        (skip-chars-forward " \t")
        (when-let* ((first (mmix--forward-word)))
          (let* ((second (mmix--forward-word))
                 (operation-first (mmix--operation-first-p
                                   (mmix--bounds-text first)
                                   (mmix--bounds-text second)
                                   (mmix--operands-at-point-p)
                                   indented))
                 (label (unless operation-first first))
                 (operation (if operation-first first second)))
            (when operation
              (goto-char (cdr operation))
              (skip-chars-forward " \t"))
            (mmix--make-statement
             :label (mmix--bounds-text label)
             :label-beg (car label)
             :label-end (cdr label)
             :operation (mmix--bounds-text operation)
             :operation-beg (car operation)
             :operation-end (cdr operation)
             :operands-beg (and operation (mmix--operands-at-point-p) (point)))))))))

(defun mmix--operation-first-p (first second rest indented)
  "Return non-nil when FIRST, a line's first word, is its operation.
SECOND is the word after FIRST, or nil.  REST is non-nil when something
other than a comment follows the last of the two words.  INDENTED is
non-nil when the line began with a blank or a tab: checksmix reads an
indented line as having no label field, so its first word is always the
operation there, whatever it spells.

checksmix reads a line as a bare statement before it reads it as a
label followed by one, and the conditions below follow that order."
  (if indented
      t
    (let ((key (mmix--keyword-key first)))
      (cond
       ;; `2ADDU': not label-shaped.
       ((not (string-match-p mmix--label-regexp first)) t)
       ((member key mmix--rest-of-line-directives) (or second rest))
       ;; `ADD $1,$2,$3' or a lone `HALT'; a lone `Done' is a label.
       ((null second) (or rest (member key mmix--standalone-instructions)))
       ;; `Main SETL $0,1'.
       ((not (mmix--keyword-kind first)) nil)
       ;; `PUT rA,$1', `JMP Loop'.
       ((not (mmix--keyword-kind second)) t)
       ;; `JMP ADD' jumps to a label named ADD; `Add ADD $1,$2,$3' and
       ;; `Set HALT' are labelled statements.
       (t (and (not rest) (member key mmix--single-operand-keywords)))))))

(defun mmix--line-operation ()
  "Return the current line's operation word, or nil."
  (when-let* ((statement (mmix--line-statement)))
    (mmix-statement-operation statement)))

(defconst mmix--debug-line-regexp
  (rx bol (? (* (not (any " \t\n"))) (+ (any " \t")))
      "debug" (+ (any " \t")) "\"" (* (not (any "\"\n"))) "\""
      (* (any " \t")) eol)
  "A line checksmix's preprocessor expands as `debug \"text\"'.
Nothing, not even a comment, may follow the closing quote.")

(defun mmix--operation-kind (statement)
  "Return the kind of STATEMENT's operation, or nil.
`debug' is a directive only on a line checksmix's preprocessor expands."
  (when-let* ((operation (mmix-statement-operation statement))
              (kind (mmix--keyword-kind operation)))
    (unless (and (string= operation mmix-debug-directive)
                 (not (save-excursion
                        (goto-char (mmix-statement-operation-beg statement))
                        (beginning-of-line)
                        (looking-at-p mmix--debug-line-regexp))))
      kind)))

(defun mmix--definition-kind (statement)
  "Return what STATEMENT's label defines: `label', `value' or nil.
A label before one of `mmix--value-directives' names a value, and any
other label an address.  A label with a trailing colon before one of
`mmix--bare-name-directives' defines nothing, since checksmix rejects it."
  (when-let* ((label (mmix-statement-label statement)))
    (let ((operation (mmix--keyword-key
                      (or (mmix-statement-operation statement) ""))))
      (cond
       ((and (member operation mmix--bare-name-directives)
             (string-suffix-p ":" label))
        nil)
       ((member operation mmix--value-directives) 'value)
       (t 'label)))))

;;;; Syntax

(defvar mmix-mode-syntax-table
  (let ((table (make-syntax-table)))
    (modify-syntax-entry ?% "<" table)
    (modify-syntax-entry ?\n ">" table)
    (modify-syntax-entry ?\" "\"" table)
    (modify-syntax-entry ?\\ "." table)
    (modify-syntax-entry ?' "." table)
    (modify-syntax-entry ?_ "_" table)
    (dolist (char '(?\; ?: ?$ ?# ?@ ?, ?- ?. ?+ ?* ?/ ?< ?> ?& ?| ?= ?~ ?! ??))
      (modify-syntax-entry char "." table))
    table)
  "Syntax table for `mmix-mode'.")

(defconst mmix--char-literal-regexp
  (rx (group "'")
      (or (seq "\\" (any "nrt0\\'")) (not (any "'\\\n")))
      (group "'"))
  "A character literal, with its two quotes as groups 1 and 2.")

(defun mmix--syntax-propertize (start end)
  "Mark character literals between START and END as strings.
A quote inside one is otherwise punctuation, so `'%'' would open a
comment."
  (goto-char start)
  (while (re-search-forward mmix--char-literal-regexp end t)
    (unless (nth 8 (save-excursion (syntax-ppss (match-beginning 0))))
      (put-text-property (match-beginning 1) (match-end 1)
                         'syntax-table (string-to-syntax "\""))
      (put-text-property (match-beginning 2) (match-end 2)
                         'syntax-table (string-to-syntax "\"")))))

;;;; Symbol references

(defconst mmix--rescan-idle-seconds 0.2
  "Idle time after an edit before the buffer's definitions are rescanned.")

(defvar-local mmix--definitions-cache nil
  "(TICK . TABLE), where TABLE maps each name the buffer defines to its kind.
TICK is the `buffer-chars-modified-tick' TABLE was built at.")

(defvar-local mmix--definitions-timer nil
  "Idle timer that will rescan this buffer's definitions, or nil.")

(defun mmix--scan-definitions ()
  "Return a table mapping each name the buffer defines to its kind.
The kind is `mmix--definition-kind'.  A trailing colon is not part of
the name; a leading one is."
  (let ((table (make-hash-table :test #'equal)))
    (save-excursion
      (goto-char (point-min))
      (while (not (eobp))
        (when-let* ((statement (mmix--line-statement))
                    (kind (mmix--definition-kind statement)))
          (puthash (string-remove-suffix ":" (mmix-statement-label statement))
                   kind table))
        (forward-line 1)))
    table))

(defun mmix--definitions ()
  "Return the buffer's definition table.
The first call scans the buffer.  After an edit the previous table is
returned at once and a rescan is scheduled for when Emacs is idle, so
typing never waits on a scan of the whole buffer."
  (let ((tick (buffer-chars-modified-tick)))
    (cond
     ((null mmix--definitions-cache)
      (setq mmix--definitions-cache (cons tick (mmix--scan-definitions))))
     ((and (/= (car mmix--definitions-cache) tick)
           (not mmix--definitions-timer))
      (setq mmix--definitions-timer
            (run-with-idle-timer mmix--rescan-idle-seconds nil
                                 #'mmix--rescan-definitions
                                 (current-buffer)))))
    (cdr mmix--definitions-cache)))

(defun mmix--rescan-definitions (buffer)
  "Rescan BUFFER's definitions, refontifying it if the names changed.
Do nothing once BUFFER is dead or has left `mmix-mode'."
  (when (buffer-live-p buffer)
    (with-current-buffer buffer
      (when (derived-mode-p 'mmix-mode)
        (setq mmix--definitions-timer nil)
        (let ((previous (cdr mmix--definitions-cache))
              (table (mmix--scan-definitions)))
          (setq mmix--definitions-cache
                (cons (buffer-chars-modified-tick) table))
          (unless (and previous (mmix--same-names-p previous table))
            (font-lock-flush)))))))

(defun mmix--same-names-p (a b)
  "Return non-nil when tables A and B hold the same names and kinds."
  (and (= (hash-table-count a) (hash-table-count b))
       (catch 'differ
         (maphash (lambda (name kind)
                    (unless (eq (gethash name b) kind)
                      (throw 'differ nil)))
                  a)
         t)))

;;;; Font lock

(defun mmix--statement-match-data (statement)
  "Return match data for STATEMENT's highlighted fields, or nil if none.
Group 1 is a label naming an address, group 2 a label naming a value,
group 3 an instruction and group 4 a directive."
  (let* ((definition (mmix--definition-kind statement))
         (kind (mmix--operation-kind statement))
         (label (cons (mmix-statement-label-beg statement)
                      (mmix-statement-label-end statement)))
         (operation (cons (mmix-statement-operation-beg statement)
                          (mmix-statement-operation-end statement)))
         (groups (list (and (eq definition 'label) label)
                       (and (eq definition 'value) label)
                       (and (eq kind 'instruction) operation)
                       (and (eq kind 'directive) operation)))
         (present (delq nil (copy-sequence groups))))
    (when present
      (append (list (car (car present)) (cdr (car (last present))))
              (mapcan (lambda (bounds) (list (car bounds) (cdr bounds)))
                      groups)))))

(defun mmix--match-statement (limit)
  "Find the next line before LIMIT with a field to highlight.
Set the match data `mmix--statement-match-data' describes."
  (let (match-data)
    (while (and (not match-data) (< (point) limit) (not (eobp)))
      (when-let* ((statement (mmix--line-statement)))
        (setq match-data (mmix--statement-match-data statement)))
      (forward-line 1))
    (when match-data
      (set-match-data match-data)
      t)))

(defconst mmix--symbol-regexp
  (rx (? ":") symbol-start (any alpha "_") (* (any alnum "_")) symbol-end)
  "A symbol as an operand spells it, with an optional global colon.")

(defun mmix--match-reference (limit)
  "Find the next use of a name the buffer defines before LIMIT.
Group 1 matches a label naming an address, group 2 one naming a value."
  (let ((definitions (mmix--definitions))
        found)
    (while (and (not found) (re-search-forward mmix--symbol-regexp limit t))
      (let ((beg (match-beginning 0))
            (end (match-end 0)))
        (pcase (gethash (match-string-no-properties 0) definitions)
          ('label (set-match-data (list beg end beg end))
                  (setq found t))
          ('value (set-match-data (list beg end nil nil beg end))
                  (setq found t)))))
    found))

(defconst mmix--number-regexp
  (rx (or bol (not (any "$#_" alnum)))
      (group (? "-")
             (or (seq "#" (+ xdigit))
                 (seq "0" (any "xX") (+ xdigit))
                 (+ digit)))
      symbol-end)
  "A numeric literal, as group 1.")

(defconst mmix-font-lock-keywords
  `((mmix--match-statement
     (1 'font-lock-function-name-face nil t)
     (2 'font-lock-variable-name-face nil t)
     (3 'font-lock-keyword-face nil t)
     (4 'font-lock-preprocessor-face nil t))
    (,(rx "$" (+ digit) symbol-end) . 'font-lock-variable-name-face)
    (,(rx (? ":") (regexp (regexp-opt (mapcar #'car mmix-special-registers)
                                      'symbols)))
     . 'font-lock-builtin-face)
    (,(rx (? ":") (regexp (regexp-opt (mapcar #'car mmix-predefined-constants)
                                      'symbols)))
     . 'font-lock-constant-face)
    (,mmix--number-regexp 1 'font-lock-number-face)
    ("@" . 'font-lock-number-face)
    ;; A local label (`2H') in the label field, and its backward/forward
    ;; references (`2B'/`2F') in an operand. Highlighting a reference as
    ;; its jump target is out of scope; this just marks the tokens.
    (,(rx bol (* (any " \t")) (group (any "0-9") "H") symbol-end)
     1 'font-lock-function-name-face)
    (,(rx symbol-start (group (any "0-9") (any "BF")) symbol-end)
     1 'font-lock-variable-name-face)
    (mmix--match-reference
     (1 'font-lock-function-name-face nil t)
     (2 'font-lock-variable-name-face nil t)))
  "Font-lock keywords for `mmix-mode'.")

;;;; Indentation

(defun mmix-indent-line ()
  "Indent the current line into label, operation and operand fields.
Point stays with the text it was in, or moves to the indentation when
it was in the indentation."
  (interactive)
  (let ((in-indentation (<= (current-column) (current-indentation)))
        (from-end (- (point-max) (point))))
    (if-let* ((statement (mmix--line-statement)))
        (mmix--indent-statement statement)
      (mmix--indent-line-without-statement))
    (if in-indentation
        (back-to-indentation)
      (goto-char (max (point) (- (point-max) from-end))))))

(defun mmix--indent-line-without-statement ()
  "Indent a blank or comment line to `mmix-operation-column'.
A comment that starts in column 0 stays there."
  (unless (save-excursion
            (back-to-indentation)
            (and (zerop (current-column)) (looking-at-p "[%;]")))
    (indent-line-to mmix-operation-column)))

(defun mmix--indent-statement (statement)
  "Move STATEMENT's fields to their columns.
A label goes to column 0, the operation to `mmix-operation-column' and
the operands to `mmix-operand-column'.  A lone word that is not in
column 0 is taken for an operation being typed, not a label."
  (let ((label-beg (mmix-statement-label-beg statement))
        (operation (and (mmix-statement-operation-beg statement)
                        (copy-marker (mmix-statement-operation-beg statement))))
        (operands (and (mmix-statement-operands-beg statement)
                       (copy-marker (mmix-statement-operands-beg statement)))))
    (save-excursion
      (cond
       ((not label-beg)
        (mmix--reindent-line-start mmix-operation-column))
       ((not operation)
        (mmix--reindent-line-start
         (if (> label-beg (line-beginning-position)) mmix-operation-column 0)))
       (t
        (mmix--reindent-line-start 0)
        (mmix--indent-field operation mmix-operation-column)))
      (when operands
        (mmix--indent-field operands mmix-operand-column)))
    (dolist (marker (list operation operands))
      (when marker
        (set-marker marker nil)))))

(defun mmix--reindent-line-start (column)
  "Replace the current line's indentation with fresh blanks to COLUMN.
Unlike `indent-line-to', this respaces an indentation already at COLUMN,
so the result follows `indent-tabs-mode'."
  (beginning-of-line)
  (delete-horizontal-space)
  (indent-to column))

(defun mmix--indent-field (position column)
  "Replace the blanks before POSITION so the text there starts at COLUMN.
Leave one space when the text before POSITION already reaches COLUMN."
  (goto-char position)
  (delete-horizontal-space t)
  (indent-to column 1))

;;;; Help

;; MMIX's instruction set is fixed, so its reference is carried here
;; rather than read at run time; the mode needs no checksmix source tree.
(defconst mmix-instruction-reference
  '(
    (("LOC") "LOC expr"
     "Set the assembly location counter to expr")
    (("GREG") "[label] GREG expr / [label] GREG"
     "Allocate a global register initialized to expr, or to 0 if expr is omitted; optional label becomes a register alias; a nonzero value serves as a base address for the two-operand memory form")
    (("IS") "Name IS expr"
     "Define a numeric or register alias constant")
    (("PREFIX") "PREFIX str"
     "Qualify subsequent unqualified names as str<name>; names beginning with : opt out")
    (("LOCAL") "LOCAL expr"
     "Declare register expr local; checked against the global threshold at the close of assembly")
    (("BSPEC") "BSPEC expr"
     "Open special mode; only IS, PREFIX, GREG, LOCAL and the four data directives are legal until ESPEC, and their data is discarded rather than assembled")
    (("ESPEC") "ESPEC"
     "Close the special mode BSPEC opened")
    (("BYTE") "BYTE expr,..."
     "Emit one byte per operand")
    (("WYDE") "WYDE expr,..."
     "Emit one 16-bit wyde per operand")
    (("TETRA") "TETRA expr,..."
     "Emit one 32-bit tetra per operand")
    (("OCTA") "OCTA expr,..."
     "Emit one 64-bit octa per operand")
    (("INCLUDE") "INCLUDE file"
     "Assemble the named file as if inserted here, resolved relative to the including file; recursive, cycles are an error")
    (("SET") "SET $X, $Y / SET $X, imm"
     "MMIXAL alias — emits ORI $X, $Y, 0 for a register, SETL $X, imm for a wyde-wide immediate")
    (("SETI") "SETI $X, imm"
     "checksmix extension — sets a full 64-bit constant in four tetras, clearing the register")
    (("SETL") "SETL $X, YZ"
     "Set low wyde; the other 48 bits become zero")
    (("SETH") "SETH $X, YZ"
     "Set high wyde; the other 48 bits become zero")
    (("SETMH") "SETMH $X, YZ"
     "Set medium-high wyde; the other 48 bits become zero")
    (("SETML") "SETML $X, YZ"
     "Set medium-low wyde; the other 48 bits become zero")
    (("INCH") "INCH $X, YZ"
     "Add into the high wyde; the other 48 bits are preserved")
    (("INCMH") "INCMH $X, YZ"
     "Add into the medium-high wyde; a carry propagates into the high wyde")
    (("INCML") "INCML $X, YZ"
     "Add into the medium-low wyde; a carry propagates into the higher wydes")
    (("INCL") "INCL $X, YZ"
     "Add into the low wyde, unsigned wrapping; a carry propagates into the higher wydes")
    (("ORH") "ORH $X, YZ"
     "Set bits in the high wyde; the other 48 bits are preserved")
    (("ORMH") "ORMH $X, YZ"
     "Set bits in the medium-high wyde; the other 48 bits are preserved")
    (("ORML") "ORML $X, YZ"
     "Set bits in the medium-low wyde; the other 48 bits are preserved")
    (("ORL") "ORL $X, YZ"
     "Set bits in the low wyde; the other 48 bits are preserved")
    (("ANDNH") "ANDNH $X, YZ"
     "Clear bits in the high wyde; the other 48 bits are preserved")
    (("ANDNMH") "ANDNMH $X, YZ"
     "Clear bits in the medium-high wyde; the other 48 bits are preserved")
    (("ANDNML") "ANDNML $X, YZ"
     "Clear bits in the medium-low wyde; the other 48 bits are preserved")
    (("ANDNL") "ANDNL $X, YZ"
     "Clear bits in the low wyde; the other 48 bits are preserved")
    (("LDB") "LDB $X, $Y, $Z / LDB $X, $Y"
     "Load byte signed; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("LDBI" "LDB") "LDB $X, $Y, Z"
     "Load byte signed (immediate)")
    (("LDBU") "LDBU $X, $Y, $Z / LDBU $X, $Y"
     "Load byte unsigned; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("LDBUI" "LDBU") "LDBU $X, $Y, Z"
     "Load byte unsigned (immediate)")
    (("LDW") "LDW $X, $Y, $Z / LDW $X, $Y"
     "Load wyde signed; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("LDWI" "LDW") "LDW $X, $Y, Z"
     "Load wyde signed (immediate)")
    (("LDWU") "LDWU $X, $Y, $Z / LDWU $X, $Y"
     "Load wyde unsigned; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("LDWUI" "LDWU") "LDWU $X, $Y, Z"
     "Load wyde unsigned (immediate)")
    (("LDT") "LDT $X, $Y, $Z / LDT $X, $Y"
     "Load tetra signed; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("LDTI" "LDT") "LDT $X, $Y, Z"
     "Load tetra signed (immediate)")
    (("LDTU") "LDTU $X, $Y, $Z / LDTU $X, $Y"
     "Load tetra unsigned; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("LDTUI" "LDTU") "LDTU $X, $Y, Z"
     "Load tetra unsigned (immediate)")
    (("LDO") "LDO $X, $Y, $Z / LDO $X, $Y"
     "Load octa; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("LDOI" "LDO") "LDO $X, $Y, Z"
     "Load octa (immediate)")
    (("LDOU") "LDOU $X, $Y, $Z / LDOU $X, $Y"
     "Load octa unsigned; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("LDOUI" "LDOU") "LDOU $X, $Y, Z"
     "Load octa unsigned (immediate)")
    (("LDUNC") "LDUNC $X, $Y, $Z / LDUNC $X, $Y"
     "Load octa uncached; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("LDUNCI" "LDUNC") "LDUNC $X, $Y, Z"
     "Load octa uncached (immediate)")
    (("LDHT") "LDHT $X, $Y, $Z / LDHT $X, $Y"
     "Load high tetra; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("LDHTI" "LDHT") "LDHT $X, $Y, Z"
     "Load high tetra (immediate)")
    (("LDSF") "LDSF $X, $Y, $Z / LDSF $X, $Y"
     "Load short float (widen f32 → f64); the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("LDSFI" "LDSF") "LDSF $X, $Y, Z"
     "Load short float (immediate)")
    (("LDVTS") "LDVTS $X, $Y, $Z / LDVTS $X, $Y"
     "Load virtual translation status; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("LDVTSI" "LDVTS") "LDVTS $X, $Y, Z"
     "Load virtual translation status (immediate)")
    (("CSWAP") "CSWAP $X, $Y, $Z / CSWAP $X, $Y"
     "Compare and swap: if M8[$Y+$Z] = rP, store $X there and set $X ← 1; otherwise rP ← M8[$Y+$Z] and $X ← 0; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("CSWAPI" "CSWAP") "CSWAP $X, $Y, Z"
     "Compare and swap (immediate): if M8[$Y+Z] = rP, store $X there and set $X ← 1; otherwise rP ← M8[$Y+Z] and $X ← 0")
    (("LDA") "LDA $X, $Y, $Z / LDA $X, addr"
     "Load address of $Y + $Z — the ADDU $X, $Y, $Z alias; LDA $X, addr loads addr in one tetra when it fits a byte, else in four")
    (("LDAI" "LDA") "LDA $X, $Y, Z / LDAI $X, addr"
     "Load address of $Y + Z — the ADDU $X, $Y, Z alias; LDAI $X, addr loads addr in one tetra when it fits a byte, else in four")
    (("STB") "STB $X, $Y, $Z / STB $X, $Y"
     "Store byte signed; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("STBI" "STB") "STB $X, $Y, Z"
     "Store byte signed (immediate)")
    (("STBU") "STBU $X, $Y, $Z / STBU $X, $Y"
     "Store byte unsigned; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("STBUI" "STBU") "STBU $X, $Y, Z"
     "Store byte unsigned (immediate)")
    (("STW") "STW $X, $Y, $Z / STW $X, $Y"
     "Store wyde signed; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("STWI" "STW") "STW $X, $Y, Z"
     "Store wyde signed (immediate)")
    (("STWU") "STWU $X, $Y, $Z / STWU $X, $Y"
     "Store wyde unsigned; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("STWUI" "STWU") "STWU $X, $Y, Z"
     "Store wyde unsigned (immediate)")
    (("STT") "STT $X, $Y, $Z / STT $X, $Y"
     "Store tetra signed; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("STTI" "STT") "STT $X, $Y, Z"
     "Store tetra signed (immediate)")
    (("STTU") "STTU $X, $Y, $Z / STTU $X, $Y"
     "Store tetra unsigned; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("STTUI" "STTU") "STTU $X, $Y, Z"
     "Store tetra unsigned (immediate)")
    (("STO") "STO $X, $Y, $Z / STO $X, $Y"
     "Store octa; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("STOI" "STO") "STO $X, $Y, Z"
     "Store octa (immediate)")
    (("STOU") "STOU $X, $Y, $Z / STOU $X, $Y"
     "Store octa unsigned; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("STOUI" "STOU") "STOU $X, $Y, Z"
     "Store octa unsigned (immediate)")
    (("STUNC") "STUNC $X, $Y, $Z / STUNC $X, $Y"
     "Store octa uncached; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("STUNCI" "STUNC") "STUNC $X, $Y, Z"
     "Store octa uncached (immediate)")
    (("STCO") "STCO X, $Y, $Z / STCO X, $Y"
     "Store constant octabyte, or to the base address $Y alone resolves to; X is a byte or a register holding one")
    (("STCOI" "STCO") "STCO X, $Y, Z"
     "Store constant octabyte, immediate address (X is a byte or a register holding one)")
    (("STHT") "STHT $X, $Y, $Z / STHT $X, $Y"
     "Store high tetra; the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("STHTI" "STHT") "STHT $X, $Y, Z"
     "Store high tetra (immediate)")
    (("STSF") "STSF $X, $Y, $Z / STSF $X, $Y"
     "Store short float (narrow f64 → f32, honors rA rounding); the two-operand form's $Y is a register (offset 0) or a base-relative address")
    (("STSFI" "STSF") "STSF $X, $Y, Z"
     "Store short float (immediate)")
    (("ADD") "ADD $X, $Y, $Z"
     "Add signed (sets overflow)")
    (("ADDI" "ADD") "ADD $X, $Y, Z"
     "Add signed immediate")
    (("ADDU") "ADDU $X, $Y, $Z"
     "Add unsigned (wrapping, same as LDA)")
    (("ADDUI" "ADDU") "ADDU $X, $Y, Z"
     "Add unsigned immediate")
    (("2ADDU") "2ADDU $X, $Y, $Z"
     "$X = 2*$Y + $Z unsigned")
    (("2ADDUI" "2ADDU") "2ADDU $X, $Y, Z"
     "$X = 2*$Y + Z unsigned")
    (("4ADDU") "4ADDU $X, $Y, $Z"
     "$X = 4*$Y + $Z unsigned")
    (("4ADDUI" "4ADDU") "4ADDU $X, $Y, Z"
     "$X = 4*$Y + Z unsigned")
    (("8ADDU") "8ADDU $X, $Y, $Z"
     "$X = 8*$Y + $Z unsigned")
    (("8ADDUI" "8ADDU") "8ADDU $X, $Y, Z"
     "$X = 8*$Y + Z unsigned")
    (("16ADDU") "16ADDU $X, $Y, $Z"
     "$X = 16*$Y + $Z unsigned")
    (("16ADDUI" "16ADDU") "16ADDU $X, $Y, Z"
     "$X = 16*$Y + Z unsigned")
    (("SUB") "SUB $X, $Y, $Z"
     "Subtract signed (sets overflow)")
    (("SUBI" "SUB") "SUB $X, $Y, Z"
     "Subtract signed immediate")
    (("SUBU") "SUBU $X, $Y, $Z"
     "Subtract unsigned (wrapping)")
    (("SUBUI" "SUBU") "SUBU $X, $Y, Z"
     "Subtract unsigned immediate")
    (("NEG") "NEG $X, Y, $Z / NEG $X, $Z"
     "$X = Y − $Z signed (Y is literal; omitted Y is 0)")
    (("NEGI" "NEG") "NEG $X, Y, Z"
     "$X = Y − Z signed")
    (("NEGU") "NEGU $X, Y, $Z / NEGU $X, $Z"
     "$X = Y − $Z unsigned (omitted Y is 0)")
    (("NEGUI" "NEGU") "NEGU $X, Y, Z"
     "$X = Y − Z unsigned")
    (("MUL") "MUL $X, $Y, $Z"
     "Multiply signed")
    (("MULI" "MUL") "MUL $X, $Y, Z"
     "Multiply signed immediate")
    (("MULU") "MULU $X, $Y, $Z"
     "Multiply unsigned (high half in rH)")
    (("MULUI" "MULU") "MULU $X, $Y, Z"
     "Multiply unsigned immediate")
    (("DIV") "DIV $X, $Y, $Z"
     "Divide signed (remainder in rR)")
    (("DIVI" "DIV") "DIV $X, $Y, Z"
     "Divide signed immediate")
    (("DIVU") "DIVU $X, $Y, $Z"
     "Divide unsigned")
    (("DIVUI" "DIVU") "DIVU $X, $Y, Z"
     "Divide unsigned immediate")
    (("FCMP") "FCMP $X, $Y, $Z"
     "Floating compare: $X = −1/0/+1; unordered operands give 0 and raise I")
    (("FUN") "FUN $X, $Y, $Z"
     "Floating unordered: $X = 1 if NaN")
    (("FEQL") "FEQL $X, $Y, $Z"
     "Floating equal: $X = 1 if equal")
    (("FCMPE") "FCMPE $X, $Y, $Z"
     "Floating compare with epsilon (rE)")
    (("FUNE") "FUNE $X, $Y, $Z"
     "Floating unordered with epsilon (rE)")
    (("FEQLE") "FEQLE $X, $Y, $Z"
     "Floating equivalent with epsilon (rE)")
    (("FADD") "FADD $X, $Y, $Z"
     "Floating add (honors rA rounding)")
    (("FSUB") "FSUB $X, $Y, $Z"
     "Floating subtract (honors rA rounding)")
    (("FMUL") "FMUL $X, $Y, $Z"
     "Floating multiply (honors rA rounding)")
    (("FDIV") "FDIV $X, $Y, $Z"
     "Floating divide (honors rA rounding)")
    (("FREM") "FREM $X, $Y, $Z"
     "Floating remainder (IEEE 754 round-half-to-even); a zero remainder takes the dividend's sign")
    (("FSQRT") "FSQRT $X, $Z / FSQRT $X, Y, $Z"
     "Floating square root (honors rA rounding; Y = mode override)")
    (("FINT") "FINT $X, $Z / FINT $X, Y, $Z"
     "Round float to integer (honors rA rounding; Y = mode override)")
    (("FIX") "FIX $X, $Z / FIX $X, Y, $Z"
     "Convert float → signed integer (honors rA rounding; Y = mode override)")
    (("FIXU") "FIXU $X, $Z / FIXU $X, Y, $Z"
     "Convert float → unsigned integer, reduced mod 2^64 (honors rA rounding; Y = mode override)")
    (("FLOT") "FLOT $X, $Z / FLOT $X, Y, $Z"
     "Convert signed integer → float (honors rA rounding; Y = mode override)")
    (("FLOTI" "FLOT") "FLOT $X, Z / FLOT $X, Y, Z"
     "Convert signed integer → float immediate (honors rA rounding; Y = mode override)")
    (("FLOTU") "FLOTU $X, $Z / FLOTU $X, Y, $Z"
     "Convert unsigned integer → float (honors rA rounding; Y = mode override)")
    (("FLOTUI" "FLOTU") "FLOTU $X, Z / FLOTU $X, Y, Z"
     "Convert unsigned integer → float immediate (honors rA rounding; Y = mode override)")
    (("SFLOT") "SFLOT $X, $Z / SFLOT $X, Y, $Z"
     "Convert signed integer → short float (honors rA rounding; Y = mode override)")
    (("SFLOTI" "SFLOT") "SFLOT $X, Z / SFLOT $X, Y, Z"
     "Convert signed integer → short float immediate (honors rA rounding; Y = mode override)")
    (("SFLOTU") "SFLOTU $X, $Z / SFLOTU $X, Y, $Z"
     "Convert unsigned integer → short float (honors rA rounding; Y = mode override)")
    (("SFLOTUI" "SFLOTU") "SFLOTU $X, Z / SFLOTU $X, Y, Z"
     "Convert unsigned integer → short float immediate (honors rA rounding; Y = mode override)")
    (("CMP") "CMP $X, $Y, $Z"
     "Compare signed: $X = −1/0/+1")
    (("CMPI" "CMP") "CMP $X, $Y, Z"
     "Compare signed immediate")
    (("CMPU") "CMPU $X, $Y, $Z"
     "Compare unsigned: $X = −1/0/+1")
    (("CMPUI" "CMPU") "CMPU $X, $Y, Z"
     "Compare unsigned immediate")
    (("AND") "AND $X, $Y, $Z"
     "Bitwise AND")
    (("ANDI" "AND") "AND $X, $Y, Z"
     "Bitwise AND immediate")
    (("OR") "OR $X, $Y, $Z"
     "Bitwise OR")
    (("ORI" "OR") "OR $X, $Y, Z"
     "Bitwise OR immediate")
    (("XOR") "XOR $X, $Y, $Z"
     "Bitwise XOR")
    (("XORI" "XOR") "XOR $X, $Y, Z"
     "Bitwise XOR immediate")
    (("ANDN") "ANDN $X, $Y, $Z"
     "Bitwise AND-NOT ($Y & ~$Z)")
    (("ANDNI" "ANDN") "ANDN $X, $Y, Z"
     "Bitwise AND-NOT immediate")
    (("ORN") "ORN $X, $Y, $Z"
     "Bitwise OR-NOT ($Y | ~$Z)")
    (("ORNI" "ORN") "ORN $X, $Y, Z"
     "Bitwise OR-NOT immediate")
    (("NAND") "NAND $X, $Y, $Z"
     "Bitwise NAND")
    (("NANDI" "NAND") "NAND $X, $Y, Z"
     "Bitwise NAND immediate")
    (("NOR") "NOR $X, $Y, $Z"
     "Bitwise NOR")
    (("NORI" "NOR") "NOR $X, $Y, Z"
     "Bitwise NOR immediate")
    (("NXOR") "NXOR $X, $Y, $Z"
     "Bitwise XNOR")
    (("NXORI" "NXOR") "NXOR $X, $Y, Z"
     "Bitwise XNOR immediate")
    (("MUX") "MUX $X, $Y, $Z"
     "Bitwise multiplex using rM mask")
    (("MUXI" "MUX") "MUX $X, $Y, Z"
     "Bitwise multiplex immediate")
    (("BDIF") "BDIF $X, $Y, $Z"
     "Byte difference (saturating, each byte)")
    (("BDIFI" "BDIF") "BDIF $X, $Y, Z"
     "Byte difference immediate")
    (("WDIF") "WDIF $X, $Y, $Z"
     "Wyde difference (saturating)")
    (("WDIFI" "WDIF") "WDIF $X, $Y, Z"
     "Wyde difference immediate")
    (("TDIF") "TDIF $X, $Y, $Z"
     "Tetra difference (saturating)")
    (("TDIFI" "TDIF") "TDIF $X, $Y, Z"
     "Tetra difference immediate")
    (("ODIF") "ODIF $X, $Y, $Z"
     "Octa difference (saturating)")
    (("ODIFI" "ODIF") "ODIF $X, $Y, Z"
     "Octa difference immediate")
    (("SADD") "SADD $X, $Y, $Z"
     "Sideways add (population count of $Y & ~$Z)")
    (("SADDI" "SADD") "SADD $X, $Y, Z"
     "Sideways add immediate")
    (("MOR") "MOR $X, $Y, $Z"
     "Matrix OR (boolean 8×8 matrix multiply)")
    (("MORI" "MOR") "MOR $X, $Y, Z"
     "Matrix OR immediate")
    (("MXOR") "MXOR $X, $Y, $Z"
     "Matrix XOR")
    (("MXORI" "MXOR") "MXOR $X, $Y, Z"
     "Matrix XOR immediate")
    (("SL") "SL $X, $Y, $Z"
     "Shift left (signed, sets overflow)")
    (("SLI" "SL") "SL $X, $Y, Z"
     "Shift left immediate")
    (("SLU") "SLU $X, $Y, $Z"
     "Shift left unsigned")
    (("SLUI" "SLU") "SLU $X, $Y, Z"
     "Shift left unsigned immediate")
    (("SR") "SR $X, $Y, $Z"
     "Shift right signed (arithmetic)")
    (("SRI" "SR") "SR $X, $Y, Z"
     "Shift right signed immediate")
    (("SRU") "SRU $X, $Y, $Z"
     "Shift right unsigned (logical)")
    (("SRUI" "SRU") "SRU $X, $Y, Z"
     "Shift right unsigned immediate")
    (("JMP") "JMP addr"
     "Unconditional jump (24-bit relative offset)")
    (("JMPB") "JMPB addr"
     "Unconditional jump, backward target required")
    (("BN") "BN $X, addr"
     "Branch if $X < 0")
    (("BNB") "BNB $X, addr"
     "Branch if $X < 0 (backward hint)")
    (("BZ") "BZ $X, addr"
     "Branch if $X == 0")
    (("BZB") "BZB $X, addr"
     "Branch if $X == 0 (backward hint)")
    (("BP") "BP $X, addr"
     "Branch if $X > 0")
    (("BPB") "BPB $X, addr"
     "Branch if $X > 0 (backward hint)")
    (("BOD") "BOD $X, addr"
     "Branch if $X is odd")
    (("BODB") "BODB $X, addr"
     "Branch if $X is odd (backward hint)")
    (("BNN") "BNN $X, addr"
     "Branch if $X >= 0")
    (("BNNB") "BNNB $X, addr"
     "Branch if $X >= 0 (backward hint)")
    (("BNZ") "BNZ $X, addr"
     "Branch if $X != 0")
    (("BNZB") "BNZB $X, addr"
     "Branch if $X != 0 (backward hint)")
    (("BNP") "BNP $X, addr"
     "Branch if $X <= 0")
    (("BNPB") "BNPB $X, addr"
     "Branch if $X <= 0 (backward hint)")
    (("BEV") "BEV $X, addr"
     "Branch if $X is even")
    (("BEVB") "BEVB $X, addr"
     "Branch if $X is even (backward hint)")
    (("PBN") "PBN $X, Y, Z"
     "Probable branch if negative")
    (("PBNB") "PBNB $X, Y, Z"
     "Probable branch if negative (backward)")
    (("PBZ") "PBZ $X, Y, Z"
     "Probable branch if zero")
    (("PBZB") "PBZB $X, Y, Z"
     "Probable branch if zero (backward)")
    (("PBP") "PBP $X, Y, Z"
     "Probable branch if positive")
    (("PBPB") "PBPB $X, Y, Z"
     "Probable branch if positive (backward)")
    (("PBOD") "PBOD $X, Y, Z"
     "Probable branch if odd")
    (("PBODB") "PBODB $X, Y, Z"
     "Probable branch if odd (backward)")
    (("PBNN") "PBNN $X, Y, Z"
     "Probable branch if non-negative")
    (("PBNNB") "PBNNB $X, Y, Z"
     "Probable branch if non-negative (backward)")
    (("PBNZ") "PBNZ $X, Y, Z"
     "Probable branch if non-zero")
    (("PBNZB") "PBNZB $X, Y, Z"
     "Probable branch if non-zero (backward)")
    (("PBNP") "PBNP $X, Y, Z"
     "Probable branch if non-positive")
    (("PBNPB") "PBNPB $X, Y, Z"
     "Probable branch if non-positive (backward)")
    (("PBEV") "PBEV $X, Y, Z"
     "Probable branch if even")
    (("PBEVB") "PBEVB $X, Y, Z"
     "Probable branch if even (backward)")
    (("CSN") "CSN $X, $Y, $Z"
     "Conditional set if $Y < 0")
    (("CSNI") "CSNI $X, $Y, Z"
     "Conditional set if $Y < 0 (immediate)")
    (("CSZ") "CSZ $X, $Y, $Z"
     "Conditional set if $Y == 0")
    (("CSZI") "CSZI $X, $Y, Z"
     "Conditional set if $Y == 0 (immediate)")
    (("CSP") "CSP $X, $Y, $Z"
     "Conditional set if $Y > 0")
    (("CSPI") "CSPI $X, $Y, Z"
     "Conditional set if $Y > 0 (immediate)")
    (("CSOD") "CSOD $X, $Y, $Z"
     "Conditional set if $Y is odd")
    (("CSODI") "CSODI $X, $Y, Z"
     "Conditional set if $Y is odd (immediate)")
    (("CSNN") "CSNN $X, $Y, $Z"
     "Conditional set if $Y >= 0")
    (("CSNNI") "CSNNI $X, $Y, Z"
     "Conditional set if $Y >= 0 (immediate)")
    (("CSNZ") "CSNZ $X, $Y, $Z"
     "Conditional set if $Y != 0")
    (("CSNZI") "CSNZI $X, $Y, Z"
     "Conditional set if $Y != 0 (immediate)")
    (("CSNP") "CSNP $X, $Y, $Z"
     "Conditional set if $Y <= 0")
    (("CSNPI") "CSNPI $X, $Y, Z"
     "Conditional set if $Y <= 0 (immediate)")
    (("CSEV") "CSEV $X, $Y, $Z"
     "Conditional set if $Y is even")
    (("CSEVI") "CSEVI $X, $Y, Z"
     "Conditional set if $Y is even (immediate)")
    (("ZSN") "ZSN $X, $Y, $Z"
     "Zero or set $Z into $X if $Y < 0")
    (("ZSNI") "ZSNI $X, $Y, Z"
     "Zero or set immediate if $Y < 0")
    (("ZSZ") "ZSZ $X, $Y, $Z"
     "Zero or set if $Y == 0")
    (("ZSZI") "ZSZI $X, $Y, Z"
     "Zero or set immediate if $Y == 0")
    (("ZSP") "ZSP $X, $Y, $Z"
     "Zero or set if $Y > 0")
    (("ZSPI") "ZSPI $X, $Y, Z"
     "Zero or set immediate if $Y > 0")
    (("ZSOD") "ZSOD $X, $Y, $Z"
     "Zero or set if $Y is odd")
    (("ZSODI") "ZSODI $X, $Y, Z"
     "Zero or set immediate if $Y is odd")
    (("ZSNN") "ZSNN $X, $Y, $Z"
     "Zero or set if $Y >= 0")
    (("ZSNNI") "ZSNNI $X, $Y, Z"
     "Zero or set immediate if $Y >= 0")
    (("ZSNZ") "ZSNZ $X, $Y, $Z"
     "Zero or set if $Y != 0")
    (("ZSNZI") "ZSNZI $X, $Y, Z"
     "Zero or set immediate if $Y != 0")
    (("ZSNP") "ZSNP $X, $Y, $Z"
     "Zero or set if $Y <= 0")
    (("ZSNPI") "ZSNPI $X, $Y, Z"
     "Zero or set immediate if $Y <= 0")
    (("ZSEV") "ZSEV $X, $Y, $Z"
     "Zero or set if $Y is even")
    (("ZSEVI") "ZSEVI $X, $Y, Z"
     "Zero or set immediate if $Y is even")
    (("PUSHJ") "PUSHJ X, addr"
     "Push registers and jump; return address in rJ; X is a byte or a register holding one")
    (("PUSHJB") "PUSHJB X, addr"
     "Push registers and jump (backward hint); X is a byte or a register holding one")
    (("PUSHGO") "PUSHGO X, $Y, $Z / PUSHGO X, $Y"
     "Push registers and jump to $Y + $Z, or to the base address $Y alone resolves to; X is a byte or a register holding one")
    (("PUSHGOI" "PUSHGO") "PUSHGO X, $Y, Z"
     "Push registers and jump to $Y + Z; X is a byte or a register holding one")
    (("POP") "POP X, YZ / POP xyz / POP"
     "Pop registers and return; the hole gets the last of the X returned values, the rest land above it in order; XYZ=xyz for the one-operand form; a bare POP is POP 0,0")
    (("GO") "GO $X, $Y, $Z / GO $X, $Y"
     "Jump to $Y + $Z, or to the base address $Y alone resolves to; save next PC in $X")
    (("GOI" "GO") "GO $X, $Y, Z"
     "Jump to $Y + Z; save next PC in $X")
    (("GETA") "GETA $X, addr"
     "Get relative address into $X")
    (("GETAB") "GETAB $X, addr"
     "Get relative address (backward hint)")
    (("GET") "GET $X, Z"
     "Read special register Z into $X")
    (("PUT") "PUT X, $Z"
     "Write $Z into special register X")
    (("PUTI" "PUT") "PUT X, Z"
     "Write immediate Z into special register X")
    (("SAVE") "SAVE $X, 0"
     "Save register stack to memory")
    (("UNSAVE") "UNSAVE 0, $Z / UNSAVE $Z"
     "Restore register stack from memory; the one-operand form is UNSAVE 0,$Z; bare UNSAVE is an error")
    (("RESUME") "RESUME XYZ / RESUME"
     "Resume after interrupt or trip; a bare RESUME is RESUME 0")
    (("TRAP") "TRAP X, Y, Z / TRAP X, YZ / TRAP XYZ / TRAP"
     "System call (see TRAP interface above); X, Y and Z (or X) are each a pure byte or a register; a bare TRAP is TRAP 0,0,0")
    (("HALT") "HALT"
     "checksmix extension — encodes as TRAP 0,Halt,0")
    (("TRIP") "TRIP X, Y, Z / TRIP X, YZ / TRIP XYZ / TRIP"
     "Forced trip (software interrupt); X, Y and Z (or X) are each a pure byte or a register; a bare TRIP is TRIP 0,0,0")
    (("SYNC") "SYNC XYZ / SYNC"
     "Synchronize memory/pipeline; a bare SYNC is SYNC 0")
    (("SWYM") "SWYM / SWYM X / SWYM X, YZ / SWYM X, Y, Z"
     "Sympathize with your machinery (no-op); operands optional, default to zero; X, Y and Z are each a pure byte or a register")
    (("PRELD") "PRELD X, $Y, $Z / PRELD X, $Y"
     "Prefetch data into cache, or into the range the base address $Y alone resolves to; X is a byte or a register holding one")
    (("PRELDI" "PRELD") "PRELD X, $Y, Z"
     "Prefetch data (immediate); X is a byte or a register holding one")
    (("PREGO") "PREGO X, $Y, $Z / PREGO X, $Y"
     "Prefetch for execution, or for the base address $Y alone resolves to; X is a byte or a register holding one")
    (("PREGOI" "PREGO") "PREGO X, $Y, Z"
     "Prefetch for execution (immediate); X is a byte or a register holding one")
    (("PREST") "PREST X, $Y, $Z / PREST X, $Y"
     "Prestore data, or the range the base address $Y alone resolves to; X is a byte or a register holding one")
    (("PRESTI" "PREST") "PREST X, $Y, Z"
     "Prestore data (immediate); X is a byte or a register holding one")
    (("SYNCD") "SYNCD X, $Y, $Z / SYNCD X, $Y"
     "Synchronize data cache, or the range the base address $Y alone resolves to; X is a byte or a register holding one")
    (("SYNCDI" "SYNCD") "SYNCD X, $Y, Z"
     "Synchronize data cache (immediate); X is a byte or a register holding one")
    (("SYNCID") "SYNCID X, $Y, $Z / SYNCID X, $Y"
     "Synchronize instruction and data cache, or the range the base address $Y alone resolves to; X is a byte or a register holding one")
    (("SYNCIDI" "SYNCID") "SYNCID X, $Y, Z"
     "Synchronize instruction and data cache (immediate); X is a byte or a register holding one")
    (("debug") "debug \"text\""
     "checksmix preprocessor line: print text and a newline to StdOut, preserving registers"))
  "One row per instruction form and directive: (SPELLINGS SYNTAX DESCRIPTION).
SPELLINGS are the keywords the row documents, spelled as
`mmix--keyword-key' returns them.  SYNTAX lists the row's source forms,
separated by \" / \".")

(defconst mmix--instruction-help
  (let ((table (make-hash-table :test #'equal)))
    (pcase-dolist (`(,spellings ,syntax ,description) mmix-instruction-reference)
      (dolist (spelling spellings)
        (puthash spelling
                 (append (gethash spelling table)
                         (list (cons syntax description)))
                 table)))
    table)
  "Map from a keyword to the (SYNTAX . DESCRIPTION) entries documenting it.")

(defun mmix-instruction-help (word)
  "Return the (SYNTAX . DESCRIPTION) entries documenting WORD."
  (gethash (mmix--keyword-key word) mmix--instruction-help))

(defun mmix-symbol-help (symbol)
  "Return a one-line description of predefined SYMBOL, or nil."
  (let ((name (string-remove-prefix ":" symbol)))
    (if-let* ((register (assoc name mmix-special-registers)))
        (format "%s: special register %d, %s"
                name (nth 1 register) (nth 2 register))
      (when-let* ((constant (assoc name mmix-predefined-constants)))
        (format "%s: %s" name (cdr constant))))))

(defun mmix-eldoc-function (callback &rest _)
  "Document the predefined symbol at point or the line's instruction.
Report the text through eldoc's CALLBACK."
  (let ((symbol (thing-at-point 'symbol t)))
    (if-let* ((help (and symbol (mmix-symbol-help symbol))))
        (funcall callback help :thing symbol)
      (when-let* ((operation (mmix--line-operation))
                  (entries (mmix-instruction-help operation)))
        (funcall callback
                 (mapconcat (lambda (entry)
                              (format "%s: %s" (car entry) (cdr entry)))
                            entries "\n")
                 :thing (upcase operation)
                 :face 'font-lock-keyword-face)))))

(defun mmix-describe-instruction (word)
  "Describe the MMIX instruction or directive WORD in a help buffer."
  (interactive
   (let ((default (mmix--line-operation)))
     (list (completing-read
            (format-prompt "Describe instruction" default)
            (append mmix-instructions mmix-directives
                    (list mmix-debug-directive))
            nil nil nil nil default))))
  (let ((entries (mmix-instruction-help word)))
    (unless entries
      (user-error "No help for %s" word))
    (help-setup-xref (list #'mmix-describe-instruction word)
                     (called-interactively-p 'interactive))
    (with-help-window (help-buffer)
      (dolist (entry entries)
        (princ (format "%s\n    %s\n\n" (car entry) (cdr entry)))))))

;;;; checksmix

(defun mmix--run-command (file)
  "Return the shell command that runs FILE through checksmix."
  (format "%s run %s" mmix-checksmix-program (shell-quote-argument file)))

(defun mmix-run ()
  "Assemble and run the current file with `checksmix run' under `compile'."
  (interactive)
  (unless buffer-file-name
    (user-error "Buffer is not visiting a file"))
  (save-buffer)
  (compile (mmix--run-command buffer-file-name)))

;;;; Mode

(defun mmix-imenu-index ()
  "Return an imenu index of the buffer's labels."
  (let (index)
    (save-excursion
      (goto-char (point-min))
      (while (not (eobp))
        (when-let* ((statement (mmix--line-statement))
                    (label (mmix-statement-label statement)))
          (push (cons (string-remove-suffix ":" label)
                      (copy-marker (mmix-statement-label-beg statement)))
                index))
        (forward-line 1)))
    (nreverse index)))

(defvar-keymap mmix-mode-map
  "C-c C-c" #'mmix-run
  "C-c C-d" #'mmix-describe-instruction)

;;;###autoload
(define-derived-mode mmix-mode prog-mode "MMIX"
  "Major mode for checksmix MMIXAL source.

\\{mmix-mode-map}"
  (setq-local comment-start "% ")
  (setq-local comment-start-skip "%+[ \t]*")
  (setq-local comment-column 40)
  (setq-local syntax-propertize-function #'mmix--syntax-propertize)
  (setq-local font-lock-defaults '(mmix-font-lock-keywords nil nil))
  (setq-local indent-line-function #'mmix-indent-line)
  (setq-local indent-tabs-mode t)
  (setq-local tab-width 8)
  (setq-local imenu-create-index-function #'mmix-imenu-index)
  (add-hook 'eldoc-documentation-functions #'mmix-eldoc-function nil t)
  (when buffer-file-name
    (setq-local compile-command (mmix--run-command buffer-file-name))))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.mms\\'" . mmix-mode))

(provide 'mmix-mode)
;;; mmix-mode.el ends here
