;;; mmix-mode.el --- Major mode for checksmix MMIXAL source -*- lexical-binding: t; -*-

;; Package-Requires: ((emacs "29.1"))

;;; Commentary:

;; Syntax highlighting, indentation and instruction help for `.mms' files,
;; following the MMIXAL dialect checksmix assembles rather than Knuth's.
;; Where the two differ this mode follows checksmix:
;;
;;   - `%' and `;' both start a comment that runs to the end of the line.
;;   - A string literal has no escapes; a character literal accepts
;;     \n \r \t \0 \\ and \'.
;;   - A statement is not column-sensitive.  Its first word is a label
;;     unless it is a mnemonic or directive, and a label may carry a
;;     leading `:' (global) and a trailing `:'.
;;   - Mnemonics and directives are case-insensitive; predefined symbols
;;     (`rJ', `StdOut', `Fputs', `ROUND_NEAR', ...) are case-sensitive.
;;   - The explicit immediate spellings (ADDI, SETI, GETAB, ...), the
;;     extensions JE/JNE/JL/JG/HALT, `.BYTE'-style directives, QUAD,
;;     INCLUDE and the `debug "text"' preprocessor line are all keywords.
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
    "GO" "GOI" "HALT" "INCH" "INCL" "INCMH" "INCML" "JE" "JG" "JL" "JMP"
    "JMPB" "JNE" "LDA" "LDAI" "LDB" "LDBI" "LDBU" "LDBUI" "LDHT" "LDHTI"
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
  '("BYTE" ".BYTE" "WYDE" ".WYDE" "TETRA" ".TETRA" "OCTA" ".OCTA"
    "QUAD" ".QUAD" "LOC" "GREG" "IS" "PREFIX" "INCLUDE" ".INCLUDE")
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
    ("Halt"
     . "TRAP function 0: Stop execution, exit code in $255; $255 = exit code")
    ("Trip"
     . "TRAP function 1: Cause a forced trip")
    ("Fopen"
     . "TRAP function 2: Open a file; $255 = filename ptr, $0 = mode; returns fd in $255")
    ("Fclose"
     . "TRAP function 3: Close a file descriptor; $255 = fd")
    ("Fread"
     . "TRAP function 4: Read bytes from fd; $255 = fd, $0 = buf ptr, $1 = count; returns bytes read")
    ("Fgets"
     . "TRAP function 5: Read a line (null-terminated) from fd; $255 = fd, $0 = buf ptr, $1 = max bytes")
    ("Fgetws"
     . "TRAP function 6: Read a wide string from fd; $255 = fd, $0 = buf ptr, $1 = max wydes")
    ("Fwrite"
     . "TRAP function 7: Write bytes to fd; $255 = fd, $0 = buf ptr, $1 = count; returns bytes written")
    ("Fputs"
     . "TRAP function 8: Write null-terminated string to fd; $255 = fd, $0 = string ptr; bytes ≥ 0x80 emitted raw")
    ("Fputc"
     . "TRAP function 9: Write one byte to fd; $255 = fd, $0 = byte; high byte of $0 emitted raw")
    ("Fputws"
     . "TRAP function 10: Write null-terminated wide string to fd; $255 = fd, $0 = string ptr")
    ("Fseek"
     . "TRAP function 11: Seek within fd; $255 = fd, $0 = offset, $1 = whence")
    ("Ftell"
     . "TRAP function 12: Get current position in fd; $255 = fd; returns position in $255")
    ("Time"
     . "TRAP function 13: Current time; returns microseconds since Unix epoch in $255")
    ("ROUND_CURRENT" . "rounding-mode override 0: use rA's mode")
    ("ROUND_OFF" . "rounding-mode override 1: toward zero")
    ("ROUND_UP" . "rounding-mode override 2: toward +infinity")
    ("ROUND_DOWN" . "rounding-mode override 3: toward -infinity")
    ("ROUND_NEAR" . "rounding-mode override 4: to nearest, ties to even"))
  "Constant symbols checksmix predefines, other than special registers.
Each is (NAME . MEANING).")

(defconst mmix--keywords
  (let ((table (make-hash-table :test #'equal)))
    (dolist (name mmix-instructions) (puthash name 'instruction table))
    (dolist (name mmix-directives) (puthash name 'directive table))
    table)
  "Map from upper-case mnemonic or directive to its kind.")

(defun mmix--keyword-kind (word)
  "Return `instruction', `directive' or nil for WORD."
  (if (string= word mmix-debug-directive)
      'directive
    (gethash (upcase word) mmix--keywords)))

;;;; Statement fields

(defconst mmix--word-regexp "[.:]?[[:alnum:]_]+:?"
  "A word that may stand in a statement's label or operation field.")

(defconst mmix--label-regexp "\\`:?[[:alpha:]_][[:alnum:]_]*:?\\'"
  "A label: an optionally global symbol with an optional trailing colon.")

(defconst mmix--single-operand-keywords
  '("JMP" "JMPB" "RESUME" "SYNC" "LOC" "GREG" "PREFIX" "BYTE" ".BYTE"
    "WYDE" ".WYDE" "TETRA" ".TETRA" "OCTA" ".OCTA" "QUAD" ".QUAD")
  "Keywords whose statement is complete with a single operand.")

(defun mmix--line-fields ()
  "Return the statement fields of the current line, or nil.
The value is (LABEL-BEG LABEL-END OP-BEG OP-END OPERAND-BEG); a field
absent from the line is nil.  `mmix--operation-first-p' decides whether
the first word is a label or the operation."
  (save-excursion
    (beginning-of-line)
    (unless (nth 3 (syntax-ppss))
      (skip-chars-forward " \t")
      (when (looking-at mmix--word-regexp)
        (let* ((beg1 (match-beginning 0))
               (end1 (goto-char (match-end 0)))
               (word1 (match-string-no-properties 0))
               (beg2 (progn (skip-chars-forward " \t") (point)))
               (end2 (and (looking-at mmix--word-regexp) (match-end 0)))
               (word2 (and end2 (buffer-substring-no-properties beg2 end2)))
               (rest (progn (when end2
                              (goto-char end2)
                              (skip-chars-forward " \t"))
                            (mmix--operands-at-point-p)))
               (op-first (mmix--operation-first-p word1 word2 rest))
               (op-beg (if op-first beg1 (and end2 beg2)))
               (op-end (if op-first end1 end2)))
          (when op-end
            (goto-char op-end)
            (skip-chars-forward " \t"))
          (list (unless op-first beg1) (unless op-first end1) op-beg op-end
                (and op-end (mmix--operands-at-point-p) (point))))))))

(defun mmix--operation-first-p (word1 word2 rest)
  "Return non-nil when WORD1, a line's first word, is its operation.
WORD2 is the word after WORD1, or nil.  REST is non-nil when something
other than a comment follows the last of the two words.

checksmix reads a line as a bare statement before it reads it as a
label followed by one, and the conditions below follow that order."
  (let ((name1 (upcase word1))
        (kind1 (mmix--keyword-kind word1)))
    (cond
     ;; `.BYTE', `2ADDU': not label-shaped.
     ((not (string-match-p mmix--label-regexp word1)) t)
     ;; INCLUDE takes the rest of the line, whatever it spells.
     ((string= name1 "INCLUDE") (or word2 rest))
     ;; `ADD $1,$2,$3' or a lone `HALT'; a lone `Done' is a label.
     ((null word2) (or rest (member name1 '("HALT" "SWYM"))))
     ;; `Main SETL $0,1'.
     ((not kind1) nil)
     ;; `PUT rA,$1', `JMP Loop'.
     ((not (mmix--keyword-kind word2)) t)
     ;; `JMP ADD' jumps to a label named ADD; `Add ADD $1,$2,$3' and
     ;; `Set HALT' are labelled statements.
     (t (and (not rest) (member name1 mmix--single-operand-keywords))))))

(defun mmix--operands-at-point-p ()
  "Return non-nil when point is before text that is not a comment."
  (not (or (eolp) (looking-at-p "[%;]"))))

(defun mmix--line-operation ()
  "Return the current line's operation word, or nil."
  (pcase (mmix--line-fields)
    (`(,_ ,_ ,beg ,end ,_)
     (and beg (buffer-substring-no-properties beg end)))))

;;;; Syntax

(defvar mmix-mode-syntax-table
  (let ((table (make-syntax-table)))
    (modify-syntax-entry ?% "<" table)
    (modify-syntax-entry ?\; "<" table)
    (modify-syntax-entry ?\n ">" table)
    (modify-syntax-entry ?\" "\"" table)
    (modify-syntax-entry ?\\ "." table)
    (modify-syntax-entry ?' "." table)
    (modify-syntax-entry ?_ "_" table)
    (dolist (char '(?: ?$ ?# ?@ ?, ?- ?. ?+ ?* ?/ ?< ?> ?& ?| ?= ?~ ?! ??))
      (modify-syntax-entry char "." table))
    table)
  "Syntax table for `mmix-mode'.")

(defconst mmix--char-literal-regexp
  "\\('\\)\\(?:\\\\[nrt0\\\\']\\|[^'\\\\\n]\\)\\('\\)"
  "A character literal, with its two quotes as groups 1 and 2.")

(defun mmix--syntax-propertize (start end)
  "Mark character literals between START and END as strings.
A quote inside one is otherwise punctuation, so `'%'' and `';'' would
open a comment."
  (goto-char start)
  (while (re-search-forward mmix--char-literal-regexp end t)
    (unless (nth 8 (save-excursion (syntax-ppss (match-beginning 0))))
      (put-text-property (match-beginning 1) (match-end 1)
                         'syntax-table (string-to-syntax "\""))
      (put-text-property (match-beginning 2) (match-end 2)
                         'syntax-table (string-to-syntax "\"")))))

;;;; Font lock

(defconst mmix--debug-line-regexp
  "^\\(?:[^ \t\n]*[ \t]+\\)?debug[ \t]+\"[^\"\n]*\"[ \t]*$"
  "A line checksmix's preprocessor expands as `debug \"text\"'.
Nothing, not even a comment, may follow the closing quote.")

(defun mmix--statement-kind (op-beg op-end)
  "Return the kind of the operation between OP-BEG and OP-END, or nil.
`debug' is a directive only on a line the preprocessor accepts."
  (let* ((operation (buffer-substring-no-properties op-beg op-end))
         (kind (mmix--keyword-kind operation)))
    (if (and (string= operation mmix-debug-directive)
             (not (save-excursion
                    (goto-char op-beg)
                    (beginning-of-line)
                    (looking-at-p mmix--debug-line-regexp))))
        nil
      kind)))

(defun mmix--match-statement (limit)
  "Find the next statement line before LIMIT and set its match data.
Group 1 is a label, group 2 an instruction, group 3 a directive."
  (let (found)
    (while (and (not found) (< (point) limit) (not (eobp)))
      (let ((fields (mmix--line-fields)))
        (forward-line 1)
        (pcase fields
          (`(,label-beg ,label-end ,op-beg ,op-end ,_)
           (let ((kind (and op-beg (mmix--statement-kind op-beg op-end))))
             (unless (and label-beg
                          (mmix--definition-kind
                           (buffer-substring-no-properties label-beg label-end)
                           (and op-beg (buffer-substring-no-properties
                                        op-beg op-end))))
               (setq label-beg nil label-end nil))
             (when (or label-beg kind)
               (set-match-data
                (list (or label-beg op-beg) (or op-end label-end)
                      label-beg label-end
                      (and (eq kind 'instruction) op-beg)
                      (and (eq kind 'instruction) op-end)
                      (and (eq kind 'directive) op-beg)
                      (and (eq kind 'directive) op-end)))
               (setq found t)))))))
    found))

(defun mmix--label-face ()
  "Face for the label matched by `mmix--match-statement'."
  (if (eq (mmix--definition-kind (match-string-no-properties 1)
                                 (match-string-no-properties 3))
          'value)
      'font-lock-variable-name-face
    'font-lock-function-name-face))

(defun mmix--definition-kind (label operation)
  "Return what LABEL defines when OPERATION follows it, or nil.
OPERATION is the word after LABEL, or nil.  A name bound by IS or GREG
is a `value' and any other label a `label'.  checksmix reads IS's name
as a bare symbol, so a label with a trailing colon before IS defines
nothing."
  (let ((operation (and operation (upcase operation))))
    (cond
     ((and (string= operation "IS") (string-suffix-p ":" label)) nil)
     ((member operation '("IS" "GREG")) 'value)
     (t 'label))))

;;;; Symbol references

(defvar-local mmix--definitions nil
  "(TICK . TABLE) mapping each name this buffer defines to its kind.
TICK is the `buffer-chars-modified-tick' TABLE was built at.")

(defvar-local mmix--definitions-timer nil
  "Idle timer that will rescan this buffer's definitions, or nil.")

(defun mmix--scan-definitions ()
  "Return a table mapping each name the buffer defines to its kind.
The kind is `mmix--definition-kind'.  A trailing colon is not part of
the name; a leading one is."
  (let ((table (make-hash-table :test #'equal)))
    (save-excursion
      (save-match-data
        (goto-char (point-min))
        (while (not (eobp))
          (pcase (mmix--line-fields)
            (`(,label-beg ,label-end ,op-beg ,op-end ,_)
             (when-let* ((label (and label-beg
                                     (buffer-substring-no-properties
                                      label-beg label-end)))
                         (kind (mmix--definition-kind
                                label
                                (and op-beg (buffer-substring-no-properties
                                             op-beg op-end)))))
               (puthash (string-remove-suffix ":" label) kind table))))
          (forward-line 1))))
    table))

(defun mmix--definitions ()
  "Return the buffer's definition table.
The first call scans the buffer.  After an edit the previous table is
returned at once and a rescan is scheduled for when Emacs is idle, so
typing never waits on a scan of the whole buffer."
  (let ((tick (buffer-chars-modified-tick)))
    (cond
     ((null mmix--definitions)
      (setq mmix--definitions (cons tick (mmix--scan-definitions))))
     ((and (not (eql (car mmix--definitions) tick))
           (not mmix--definitions-timer))
      (setq mmix--definitions-timer
            (run-with-idle-timer 0.2 nil #'mmix--rescan-definitions
                                 (current-buffer)))))
    (cdr mmix--definitions)))

(defun mmix--rescan-definitions (buffer)
  "Rescan BUFFER's definitions, refontifying it if the names changed.
Do nothing once BUFFER is dead or has left `mmix-mode'."
  (when (buffer-live-p buffer)
    (with-current-buffer buffer
      (when (derived-mode-p 'mmix-mode)
        (setq mmix--definitions-timer nil)
        (let ((previous (cdr mmix--definitions))
              (table (mmix--scan-definitions)))
          (setq mmix--definitions (cons (buffer-chars-modified-tick) table))
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

(defconst mmix--symbol-regexp ":?\\_<[[:alpha:]_][[:alnum:]_]*\\_>"
  "A symbol as an operand spells it, with an optional global colon.")

(defun mmix--match-reference (limit)
  "Find the next use of a name the buffer defines before LIMIT.
Group 1 matches a label, group 2 a name bound by IS or GREG."
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

(defconst mmix-font-lock-keywords
  `((mmix--match-statement
     (1 (mmix--label-face) nil t)
     (2 'font-lock-keyword-face nil t)
     (3 'font-lock-preprocessor-face nil t))
    ("\\$[0-9]+\\_>" . 'font-lock-variable-name-face)
    (,(concat ":?" (regexp-opt (mapcar #'car mmix-special-registers) 'symbols))
     . 'font-lock-builtin-face)
    (,(concat ":?" (regexp-opt (mapcar #'car mmix-predefined-constants) 'symbols))
     . 'font-lock-constant-face)
    ("\\(?:^\\|[^$#[:alnum:]_]\\)\\(-?\\(?:#[[:xdigit:]]+\\|0[xX][[:xdigit:]]+\\|[0-9]+\\)\\)\\_>"
     1 'font-lock-number-face)
    ("@" . 'font-lock-number-face)
    (mmix--match-reference
     (1 'font-lock-function-name-face nil t)
     (2 'font-lock-variable-name-face nil t)))
  "Font-lock keywords for `mmix-mode'.")

;;;; Indentation

(defun mmix-indent-line ()
  "Indent the current line into label, operation and operand fields.
A label goes to column 0, the operation to `mmix-operation-column' and
the operands to `mmix-operand-column'.  A lone word that is not at
column 0 is taken for an operation being typed, not a label."
  (interactive)
  (let ((offset (- (point-max) (point)))
        (in-indentation (<= (current-column) (current-indentation))))
    (pcase (mmix--line-fields)
      ('nil
       (unless (save-excursion
                 (back-to-indentation)
                 (and (looking-at "[%;]") (zerop (current-column))))
         (indent-line-to mmix-operation-column)))
      (`(,label-beg ,_ ,op-beg ,_ ,operand-beg)
       (let ((op (and op-beg (copy-marker op-beg)))
             (operands (and operand-beg (copy-marker operand-beg)))
             (lone-indented-word (and label-beg (not op-beg)
                                      (> label-beg (line-beginning-position)))))
         (save-excursion
           (beginning-of-line)
           (delete-horizontal-space)
           (cond
            (lone-indented-word
             (indent-to mmix-operation-column))
            ((not op))
            (label-beg
             (goto-char op)
             (delete-horizontal-space)
             (indent-to mmix-operation-column 1))
            (t
             (indent-to mmix-operation-column)))
           (when operands
             (goto-char operands)
             (delete-horizontal-space)
             (indent-to mmix-operand-column 1)))
         (when op (set-marker op nil))
         (when operands (set-marker operands nil)))))
    (if in-indentation
        (back-to-indentation)
      (when (> (- (point-max) offset) (point))
        (goto-char (- (point-max) offset))))))

;;;; Help

;; MMIX's instruction set is fixed, so its reference is carried here
;; rather than read at run time; the mode needs no checksmix source tree.
(defconst mmix-instruction-reference
  '(
    ("LOC" "LOC expr"
     "Set the assembly location counter to expr")
    ("GREG" "[label] GREG expr"
     "Allocate a global register initialized to expr; optional label becomes a register alias")
    ("IS" "Name IS expr"
     "Define a numeric or register alias constant")
    ("PREFIX" "PREFIX str"
     "Qualify subsequent unqualified names as str<name>; names beginning with : opt out")
    ("BYTE" "BYTE expr,..."
     "Emit one byte per operand")
    ("WYDE" "WYDE expr"
     "Emit one 16-bit wyde")
    ("TETRA" "TETRA expr"
     "Emit one 32-bit tetra")
    ("OCTA" "OCTA expr"
     "Emit one 64-bit octa")
    ("INCLUDE" "INCLUDE file"
     "Assemble the named file as if inserted here, resolved relative to the including file; recursive, cycles are an error")
    ("SET" "SET $X, $Y / SET $X, imm"
     "MMIXAL alias — emits ORI $X, $Y, 0 for a register, SETL $X, imm for a wyde-wide immediate")
    ("SETI" "SETI $X, imm"
     "checksmix extension — sets a full 64-bit constant in four tetras, clearing the register")
    ("SETL" "SETL $X, YZ"
     "Set low wyde; the other 48 bits become zero")
    ("SETH" "SETH $X, YZ"
     "Set high wyde; the other 48 bits become zero")
    ("SETMH" "SETMH $X, YZ"
     "Set medium-high wyde; the other 48 bits become zero")
    ("SETML" "SETML $X, YZ"
     "Set medium-low wyde; the other 48 bits become zero")
    ("INCH" "INCH $X, YZ"
     "Add into the high wyde; the other 48 bits are preserved")
    ("INCMH" "INCMH $X, YZ"
     "Add into the medium-high wyde; a carry propagates into the high wyde")
    ("INCML" "INCML $X, YZ"
     "Add into the medium-low wyde; a carry propagates into the higher wydes")
    ("INCL" "INCL $X, YZ"
     "Add into the low wyde, unsigned wrapping; a carry propagates into the higher wydes")
    ("ORH" "ORH $X, YZ"
     "Set bits in the high wyde; the other 48 bits are preserved")
    ("ORMH" "ORMH $X, YZ"
     "Set bits in the medium-high wyde; the other 48 bits are preserved")
    ("ORML" "ORML $X, YZ"
     "Set bits in the medium-low wyde; the other 48 bits are preserved")
    ("ORL" "ORL $X, YZ"
     "Set bits in the low wyde; the other 48 bits are preserved")
    ("ANDNH" "ANDNH $X, YZ"
     "Clear bits in the high wyde; the other 48 bits are preserved")
    ("ANDNMH" "ANDNMH $X, YZ"
     "Clear bits in the medium-high wyde; the other 48 bits are preserved")
    ("ANDNML" "ANDNML $X, YZ"
     "Clear bits in the medium-low wyde; the other 48 bits are preserved")
    ("ANDNL" "ANDNL $X, YZ"
     "Clear bits in the low wyde; the other 48 bits are preserved")
    ("LDB" "LDB $X, $Y, $Z"
     "Load byte signed")
    ("LDBI" "LDB $X, $Y, Z"
     "Load byte signed (immediate)")
    ("LDBU" "LDBU $X, $Y, $Z"
     "Load byte unsigned")
    ("LDBUI" "LDBU $X, $Y, Z"
     "Load byte unsigned (immediate)")
    ("LDW" "LDW $X, $Y, $Z"
     "Load wyde signed")
    ("LDWI" "LDW $X, $Y, Z"
     "Load wyde signed (immediate)")
    ("LDWU" "LDWU $X, $Y, $Z"
     "Load wyde unsigned")
    ("LDWUI" "LDWU $X, $Y, Z"
     "Load wyde unsigned (immediate)")
    ("LDT" "LDT $X, $Y, $Z"
     "Load tetra signed")
    ("LDTI" "LDT $X, $Y, Z"
     "Load tetra signed (immediate)")
    ("LDTU" "LDTU $X, $Y, $Z"
     "Load tetra unsigned")
    ("LDTUI" "LDTU $X, $Y, Z"
     "Load tetra unsigned (immediate)")
    ("LDO" "LDO $X, $Y, $Z"
     "Load octa")
    ("LDOI" "LDO $X, $Y, Z"
     "Load octa (immediate)")
    ("LDOU" "LDOU $X, $Y, $Z"
     "Load octa unsigned")
    ("LDOUI" "LDOU $X, $Y, Z"
     "Load octa unsigned (immediate)")
    ("LDUNC" "LDUNC $X, $Y, $Z"
     "Load octa uncached")
    ("LDUNCI" "LDUNC $X, $Y, Z"
     "Load octa uncached (immediate)")
    ("LDHT" "LDHT $X, $Y, $Z"
     "Load high tetra")
    ("LDHTI" "LDHT $X, $Y, Z"
     "Load high tetra (immediate)")
    ("LDSF" "LDSF $X, $Y, $Z"
     "Load short float (widen f32 → f64)")
    ("LDSFI" "LDSF $X, $Y, Z"
     "Load short float (immediate)")
    ("LDVTS" "LDVTS $X, $Y, $Z"
     "Load virtual translation status")
    ("LDVTSI" "LDVTS $X, $Y, Z"
     "Load virtual translation status (immediate)")
    ("CSWAP" "CSWAP $X, $Y, $Z"
     "Compare and swap: if M8[$Y+$Z] = rP, store $X there and set $X ← 1; otherwise rP ← M8[$Y+$Z] and $X ← 0")
    ("CSWAPI" "CSWAP $X, $Y, Z"
     "Compare and swap (immediate): if M8[$Y+Z] = rP, store $X there and set $X ← 1; otherwise rP ← M8[$Y+Z] and $X ← 0")
    ("LDA" "LDA $X, $Y, $Z / LDA $X, addr"
     "Load address of $Y + $Z — the ADDU $X, $Y, $Z alias; LDA $X, addr loads addr in one tetra when it fits a byte, else in four")
    ("LDAI" "LDA $X, $Y, Z / LDAI $X, addr"
     "Load address of $Y + Z — the ADDU $X, $Y, Z alias; LDAI $X, addr loads addr in one tetra when it fits a byte, else in four")
    ("STB" "STB $X, $Y, $Z"
     "Store byte signed")
    ("STBI" "STB $X, $Y, Z"
     "Store byte signed (immediate)")
    ("STBU" "STBU $X, $Y, $Z"
     "Store byte unsigned")
    ("STBUI" "STBU $X, $Y, Z"
     "Store byte unsigned (immediate)")
    ("STW" "STW $X, $Y, $Z"
     "Store wyde signed")
    ("STWI" "STW $X, $Y, Z"
     "Store wyde signed (immediate)")
    ("STWU" "STWU $X, $Y, $Z"
     "Store wyde unsigned")
    ("STWUI" "STWU $X, $Y, Z"
     "Store wyde unsigned (immediate)")
    ("STT" "STT $X, $Y, $Z"
     "Store tetra signed")
    ("STTI" "STT $X, $Y, Z"
     "Store tetra signed (immediate)")
    ("STTU" "STTU $X, $Y, $Z"
     "Store tetra unsigned")
    ("STTUI" "STTU $X, $Y, Z"
     "Store tetra unsigned (immediate)")
    ("STO" "STO $X, $Y, $Z"
     "Store octa")
    ("STOI" "STO $X, $Y, Z"
     "Store octa (immediate)")
    ("STOU" "STOU $X, $Y, $Z"
     "Store octa unsigned")
    ("STOUI" "STOU $X, $Y, Z"
     "Store octa unsigned (immediate)")
    ("STUNC" "STUNC $X, $Y, $Z"
     "Store octa uncached")
    ("STUNCI" "STUNC $X, $Y, Z"
     "Store octa uncached (immediate)")
    ("STCO" "STCO X, $Y, $Z"
     "Store constant octabyte")
    ("STCOI" "STCO X, $Y, Z"
     "Store constant octabyte (immediate)")
    ("STHT" "STHT $X, $Y, $Z"
     "Store high tetra")
    ("STHTI" "STHT $X, $Y, Z"
     "Store high tetra (immediate)")
    ("STSF" "STSF $X, $Y, $Z"
     "Store short float (narrow f64 → f32, honors rA rounding)")
    ("STSFI" "STSF $X, $Y, Z"
     "Store short float (immediate)")
    ("ADD" "ADD $X, $Y, $Z"
     "Add signed (sets overflow)")
    ("ADDI" "ADD $X, $Y, Z"
     "Add signed immediate")
    ("ADDU" "ADDU $X, $Y, $Z"
     "Add unsigned (wrapping, same as LDA)")
    ("ADDUI" "ADDU $X, $Y, Z"
     "Add unsigned immediate")
    ("ADDU2" "2ADDU $X, $Y, $Z"
     "$X = 2*$Y + $Z unsigned")
    ("ADDU2I" "2ADDU $X, $Y, Z"
     "$X = 2*$Y + Z unsigned")
    ("ADDU4" "4ADDU $X, $Y, $Z"
     "$X = 4*$Y + $Z unsigned")
    ("ADDU4I" "4ADDU $X, $Y, Z"
     "$X = 4*$Y + Z unsigned")
    ("ADDU8" "8ADDU $X, $Y, $Z"
     "$X = 8*$Y + $Z unsigned")
    ("ADDU8I" "8ADDU $X, $Y, Z"
     "$X = 8*$Y + Z unsigned")
    ("ADDU16" "16ADDU $X, $Y, $Z"
     "$X = 16*$Y + $Z unsigned")
    ("ADDU16I" "16ADDU $X, $Y, Z"
     "$X = 16*$Y + Z unsigned")
    ("SUB" "SUB $X, $Y, $Z"
     "Subtract signed (sets overflow)")
    ("SUBI" "SUB $X, $Y, Z"
     "Subtract signed immediate")
    ("SUBU" "SUBU $X, $Y, $Z"
     "Subtract unsigned (wrapping)")
    ("SUBUI" "SUBU $X, $Y, Z"
     "Subtract unsigned immediate")
    ("NEG" "NEG $X, Y, $Z"
     "$X = Y − $Z signed (Y is literal)")
    ("NEGI" "NEG $X, Y, Z"
     "$X = Y − Z signed")
    ("NEGU" "NEGU $X, Y, $Z"
     "$X = Y − $Z unsigned")
    ("NEGUI" "NEGU $X, Y, Z"
     "$X = Y − Z unsigned")
    ("MUL" "MUL $X, $Y, $Z"
     "Multiply signed")
    ("MULI" "MUL $X, $Y, Z"
     "Multiply signed immediate")
    ("MULU" "MULU $X, $Y, $Z"
     "Multiply unsigned (high half in rH)")
    ("MULUI" "MULU $X, $Y, Z"
     "Multiply unsigned immediate")
    ("DIV" "DIV $X, $Y, $Z"
     "Divide signed (remainder in rR)")
    ("DIVI" "DIV $X, $Y, Z"
     "Divide signed immediate")
    ("DIVU" "DIVU $X, $Y, $Z"
     "Divide unsigned")
    ("DIVUI" "DIVU $X, $Y, Z"
     "Divide unsigned immediate")
    ("FCMP" "FCMP $X, $Y, $Z"
     "Floating compare: $X = −1/0/+1; unordered operands give 0 and raise I")
    ("FUN" "FUN $X, $Y, $Z"
     "Floating unordered: $X = 1 if NaN")
    ("FEQL" "FEQL $X, $Y, $Z"
     "Floating equal: $X = 1 if equal")
    ("FCMPE" "FCMPE $X, $Y, $Z"
     "Floating compare with epsilon (rE)")
    ("FUNE" "FUNE $X, $Y, $Z"
     "Floating unordered with epsilon (rE)")
    ("FEQLE" "FEQLE $X, $Y, $Z"
     "Floating equivalent with epsilon (rE)")
    ("FADD" "FADD $X, $Y, $Z"
     "Floating add (honors rA rounding)")
    ("FSUB" "FSUB $X, $Y, $Z"
     "Floating subtract (honors rA rounding)")
    ("FMUL" "FMUL $X, $Y, $Z"
     "Floating multiply (honors rA rounding)")
    ("FDIV" "FDIV $X, $Y, $Z"
     "Floating divide (honors rA rounding)")
    ("FREM" "FREM $X, $Y, $Z"
     "Floating remainder (IEEE 754 round-half-to-even); a zero remainder takes the dividend's sign")
    ("FSQRT" "FSQRT $X, $Z / FSQRT $X, Y, $Z"
     "Floating square root (honors rA rounding; Y = mode override)")
    ("FINT" "FINT $X, $Z / FINT $X, Y, $Z"
     "Round float to integer (honors rA rounding; Y = mode override)")
    ("FIX" "FIX $X, $Z / FIX $X, Y, $Z"
     "Convert float → signed integer (honors rA rounding; Y = mode override)")
    ("FIXU" "FIXU $X, $Z / FIXU $X, Y, $Z"
     "Convert float → unsigned integer, reduced mod 2^64 (honors rA rounding; Y = mode override)")
    ("FLOT" "FLOT $X, $Z / FLOT $X, Y, $Z"
     "Convert signed integer → float (honors rA rounding; Y = mode override)")
    ("FLOTI" "FLOT $X, Z / FLOT $X, Y, Z"
     "Convert signed integer → float immediate (honors rA rounding; Y = mode override)")
    ("FLOTU" "FLOTU $X, $Z / FLOTU $X, Y, $Z"
     "Convert unsigned integer → float (honors rA rounding; Y = mode override)")
    ("FLOTUI" "FLOTU $X, Z / FLOTU $X, Y, Z"
     "Convert unsigned integer → float immediate (honors rA rounding; Y = mode override)")
    ("SFLOT" "SFLOT $X, $Z / SFLOT $X, Y, $Z"
     "Convert signed integer → short float (honors rA rounding; Y = mode override)")
    ("SFLOTI" "SFLOT $X, Z / SFLOT $X, Y, Z"
     "Convert signed integer → short float immediate (honors rA rounding; Y = mode override)")
    ("SFLOTU" "SFLOTU $X, $Z / SFLOTU $X, Y, $Z"
     "Convert unsigned integer → short float (honors rA rounding; Y = mode override)")
    ("SFLOTUI" "SFLOTU $X, Z / SFLOTU $X, Y, Z"
     "Convert unsigned integer → short float immediate (honors rA rounding; Y = mode override)")
    ("CMP" "CMP $X, $Y, $Z"
     "Compare signed: $X = −1/0/+1")
    ("CMPI" "CMP $X, $Y, Z"
     "Compare signed immediate")
    ("CMPU" "CMPU $X, $Y, $Z"
     "Compare unsigned: $X = −1/0/+1")
    ("CMPUI" "CMPU $X, $Y, Z"
     "Compare unsigned immediate")
    ("AND" "AND $X, $Y, $Z"
     "Bitwise AND")
    ("ANDI" "AND $X, $Y, Z"
     "Bitwise AND immediate")
    ("OR" "OR $X, $Y, $Z"
     "Bitwise OR")
    ("ORI" "OR $X, $Y, Z"
     "Bitwise OR immediate")
    ("XOR" "XOR $X, $Y, $Z"
     "Bitwise XOR")
    ("XORI" "XOR $X, $Y, Z"
     "Bitwise XOR immediate")
    ("ANDN" "ANDN $X, $Y, $Z"
     "Bitwise AND-NOT ($Y & ~$Z)")
    ("ANDNI" "ANDN $X, $Y, Z"
     "Bitwise AND-NOT immediate")
    ("ORN" "ORN $X, $Y, $Z"
     "Bitwise OR-NOT ($Y | ~$Z)")
    ("ORNI" "ORN $X, $Y, Z"
     "Bitwise OR-NOT immediate")
    ("NAND" "NAND $X, $Y, $Z"
     "Bitwise NAND")
    ("NANDI" "NAND $X, $Y, Z"
     "Bitwise NAND immediate")
    ("NOR" "NOR $X, $Y, $Z"
     "Bitwise NOR")
    ("NORI" "NOR $X, $Y, Z"
     "Bitwise NOR immediate")
    ("NXOR" "NXOR $X, $Y, $Z"
     "Bitwise XNOR")
    ("NXORI" "NXOR $X, $Y, Z"
     "Bitwise XNOR immediate")
    ("MUX" "MUX $X, $Y, $Z"
     "Bitwise multiplex using rM mask")
    ("MUXI" "MUX $X, $Y, Z"
     "Bitwise multiplex immediate")
    ("BDIF" "BDIF $X, $Y, $Z"
     "Byte difference (saturating, each byte)")
    ("BDIFI" "BDIF $X, $Y, Z"
     "Byte difference immediate")
    ("WDIF" "WDIF $X, $Y, $Z"
     "Wyde difference (saturating)")
    ("WDIFI" "WDIF $X, $Y, Z"
     "Wyde difference immediate")
    ("TDIF" "TDIF $X, $Y, $Z"
     "Tetra difference (saturating)")
    ("TDIFI" "TDIF $X, $Y, Z"
     "Tetra difference immediate")
    ("ODIF" "ODIF $X, $Y, $Z"
     "Octa difference (saturating)")
    ("ODIFI" "ODIF $X, $Y, Z"
     "Octa difference immediate")
    ("SADD" "SADD $X, $Y, $Z"
     "Sideways add (population count of $Y & ~$Z)")
    ("SADDI" "SADD $X, $Y, Z"
     "Sideways add immediate")
    ("MOR" "MOR $X, $Y, $Z"
     "Matrix OR (boolean 8×8 matrix multiply)")
    ("MORI" "MOR $X, $Y, Z"
     "Matrix OR immediate")
    ("MXOR" "MXOR $X, $Y, $Z"
     "Matrix XOR")
    ("MXORI" "MXOR $X, $Y, Z"
     "Matrix XOR immediate")
    ("SL" "SL $X, $Y, $Z"
     "Shift left (signed, sets overflow)")
    ("SLI" "SL $X, $Y, Z"
     "Shift left immediate")
    ("SLU" "SLU $X, $Y, $Z"
     "Shift left unsigned")
    ("SLUI" "SLU $X, $Y, Z"
     "Shift left unsigned immediate")
    ("SR" "SR $X, $Y, $Z"
     "Shift right signed (arithmetic)")
    ("SRI" "SR $X, $Y, Z"
     "Shift right signed immediate")
    ("SRU" "SRU $X, $Y, $Z"
     "Shift right unsigned (logical)")
    ("SRUI" "SRU $X, $Y, Z"
     "Shift right unsigned immediate")
    ("JMP" "JMP addr"
     "Unconditional jump (24-bit relative offset)")
    ("JMPB" "JMPB addr"
     "Unconditional jump, backward target required")
    ("BN" "BN $X, addr"
     "Branch if $X < 0")
    ("BNB" "BNB $X, addr"
     "Branch if $X < 0 (backward hint)")
    ("BZ" "BZ $X, addr"
     "Branch if $X == 0")
    ("BZB" "BZB $X, addr"
     "Branch if $X == 0 (backward hint)")
    ("BP" "BP $X, addr"
     "Branch if $X > 0")
    ("BPB" "BPB $X, addr"
     "Branch if $X > 0 (backward hint)")
    ("BOD" "BOD $X, addr"
     "Branch if $X is odd")
    ("BODB" "BODB $X, addr"
     "Branch if $X is odd (backward hint)")
    ("BNN" "BNN $X, addr"
     "Branch if $X >= 0")
    ("BNNB" "BNNB $X, addr"
     "Branch if $X >= 0 (backward hint)")
    ("BNZ" "BNZ $X, addr"
     "Branch if $X != 0")
    ("BNZB" "BNZB $X, addr"
     "Branch if $X != 0 (backward hint)")
    ("BNP" "BNP $X, addr"
     "Branch if $X <= 0")
    ("BNPB" "BNPB $X, addr"
     "Branch if $X <= 0 (backward hint)")
    ("BEV" "BEV $X, addr"
     "Branch if $X is even")
    ("BEVB" "BEVB $X, addr"
     "Branch if $X is even (backward hint)")
    ("JE" "JE $X, addr"
     "checksmix extension — branch if $X == 0; encodes as BZ/BZB")
    ("JNE" "JNE $X, addr"
     "checksmix extension — branch if $X != 0; encodes as BNZ/BNZB")
    ("JL" "JL $X, addr"
     "checksmix extension — branch if $X < 0; encodes as BN/BNB")
    ("JG" "JG $X, addr"
     "checksmix extension — branch if $X > 0; encodes as BP/BPB")
    ("PBN" "PBN $X, Y, Z"
     "Probable branch if negative")
    ("PBNB" "PBNB $X, Y, Z"
     "Probable branch if negative (backward)")
    ("PBZ" "PBZ $X, Y, Z"
     "Probable branch if zero")
    ("PBZB" "PBZB $X, Y, Z"
     "Probable branch if zero (backward)")
    ("PBP" "PBP $X, Y, Z"
     "Probable branch if positive")
    ("PBPB" "PBPB $X, Y, Z"
     "Probable branch if positive (backward)")
    ("PBOD" "PBOD $X, Y, Z"
     "Probable branch if odd")
    ("PBODB" "PBODB $X, Y, Z"
     "Probable branch if odd (backward)")
    ("PBNN" "PBNN $X, Y, Z"
     "Probable branch if non-negative")
    ("PBNNB" "PBNNB $X, Y, Z"
     "Probable branch if non-negative (backward)")
    ("PBNZ" "PBNZ $X, Y, Z"
     "Probable branch if non-zero")
    ("PBNZB" "PBNZB $X, Y, Z"
     "Probable branch if non-zero (backward)")
    ("PBNP" "PBNP $X, Y, Z"
     "Probable branch if non-positive")
    ("PBNPB" "PBNPB $X, Y, Z"
     "Probable branch if non-positive (backward)")
    ("PBEV" "PBEV $X, Y, Z"
     "Probable branch if even")
    ("PBEVB" "PBEVB $X, Y, Z"
     "Probable branch if even (backward)")
    ("CSN" "CSN $X, $Y, $Z"
     "Conditional set if $Y < 0")
    ("CSNI" "CSNI $X, $Y, Z"
     "Conditional set if $Y < 0 (immediate)")
    ("CSZ" "CSZ $X, $Y, $Z"
     "Conditional set if $Y == 0")
    ("CSZI" "CSZI $X, $Y, Z"
     "Conditional set if $Y == 0 (immediate)")
    ("CSP" "CSP $X, $Y, $Z"
     "Conditional set if $Y > 0")
    ("CSPI" "CSPI $X, $Y, Z"
     "Conditional set if $Y > 0 (immediate)")
    ("CSOD" "CSOD $X, $Y, $Z"
     "Conditional set if $Y is odd")
    ("CSODI" "CSODI $X, $Y, Z"
     "Conditional set if $Y is odd (immediate)")
    ("CSNN" "CSNN $X, $Y, $Z"
     "Conditional set if $Y >= 0")
    ("CSNNI" "CSNNI $X, $Y, Z"
     "Conditional set if $Y >= 0 (immediate)")
    ("CSNZ" "CSNZ $X, $Y, $Z"
     "Conditional set if $Y != 0")
    ("CSNZI" "CSNZI $X, $Y, Z"
     "Conditional set if $Y != 0 (immediate)")
    ("CSNP" "CSNP $X, $Y, $Z"
     "Conditional set if $Y <= 0")
    ("CSNPI" "CSNPI $X, $Y, Z"
     "Conditional set if $Y <= 0 (immediate)")
    ("CSEV" "CSEV $X, $Y, $Z"
     "Conditional set if $Y is even")
    ("CSEVI" "CSEVI $X, $Y, Z"
     "Conditional set if $Y is even (immediate)")
    ("ZSN" "ZSN $X, $Y, $Z"
     "Zero or set $Z into $X if $Y < 0")
    ("ZSNI" "ZSNI $X, $Y, Z"
     "Zero or set immediate if $Y < 0")
    ("ZSZ" "ZSZ $X, $Y, $Z"
     "Zero or set if $Y == 0")
    ("ZSZI" "ZSZI $X, $Y, Z"
     "Zero or set immediate if $Y == 0")
    ("ZSP" "ZSP $X, $Y, $Z"
     "Zero or set if $Y > 0")
    ("ZSPI" "ZSPI $X, $Y, Z"
     "Zero or set immediate if $Y > 0")
    ("ZSOD" "ZSOD $X, $Y, $Z"
     "Zero or set if $Y is odd")
    ("ZSODI" "ZSODI $X, $Y, Z"
     "Zero or set immediate if $Y is odd")
    ("ZSNN" "ZSNN $X, $Y, $Z"
     "Zero or set if $Y >= 0")
    ("ZSNNI" "ZSNNI $X, $Y, Z"
     "Zero or set immediate if $Y >= 0")
    ("ZSNZ" "ZSNZ $X, $Y, $Z"
     "Zero or set if $Y != 0")
    ("ZSNZI" "ZSNZI $X, $Y, Z"
     "Zero or set immediate if $Y != 0")
    ("ZSNP" "ZSNP $X, $Y, $Z"
     "Zero or set if $Y <= 0")
    ("ZSNPI" "ZSNPI $X, $Y, Z"
     "Zero or set immediate if $Y <= 0")
    ("ZSEV" "ZSEV $X, $Y, $Z"
     "Zero or set if $Y is even")
    ("ZSEVI" "ZSEVI $X, $Y, Z"
     "Zero or set immediate if $Y is even")
    ("PUSHJ" "PUSHJ $X, addr"
     "Push registers and jump; return address in rJ")
    ("PUSHJB" "PUSHJB $X, addr"
     "Push registers and jump (backward hint)")
    ("PUSHGO" "PUSHGO $X, $Y, $Z"
     "Push registers and jump to $Y + $Z")
    ("PUSHGOI" "PUSHGO $X, $Y, Z"
     "Push registers and jump to $Y + Z")
    ("POP" "POP X, YZ"
     "Pop registers and return; X values returned")
    ("GO" "GO $X, $Y, $Z"
     "Jump to $Y + $Z; save next PC in $X")
    ("GOI" "GO $X, $Y, Z"
     "Jump to $Y + Z; save next PC in $X")
    ("GETA" "GETA $X, addr"
     "Get relative address into $X")
    ("GETAB" "GETAB $X, addr"
     "Get relative address (backward hint)")
    ("GET" "GET $X, Z"
     "Read special register Z into $X")
    ("PUT" "PUT X, $Z"
     "Write $Z into special register X")
    ("PUTI" "PUT X, Z"
     "Write immediate Z into special register X")
    ("SAVE" "SAVE $X, 0"
     "Save register stack to memory")
    ("UNSAVE" "UNSAVE 0, $Z"
     "Restore register stack from memory")
    ("RESUME" "RESUME XYZ"
     "Resume after interrupt or trip")
    ("TRAP" "TRAP X, Y, Z"
     "System call (see TRAP interface above)")
    ("HALT" "HALT"
     "checksmix extension — encodes as TRAP 0,Halt,0")
    ("TRIP" "TRIP X, Y, Z"
     "Forced trip (software interrupt)")
    ("SYNC" "SYNC XYZ"
     "Synchronize memory/pipeline")
    ("SWYM" "SWYM / SWYM X, Y, Z"
     "Sympathize with your machinery (no-op); operands optional, default to zero")
    ("PRELD" "PRELD $X, $Y, $Z"
     "Prefetch data into cache")
    ("PRELDI" "PRELD $X, $Y, Z"
     "Prefetch data (immediate)")
    ("PREGO" "PREGO $X, $Y, $Z"
     "Prefetch for execution")
    ("PREGOI" "PREGO $X, $Y, Z"
     "Prefetch for execution (immediate)")
    ("PREST" "PREST $X, $Y, $Z"
     "Prestore data")
    ("PRESTI" "PREST $X, $Y, Z"
     "Prestore data (immediate)")
    ("SYNCD" "SYNCD $X, $Y, $Z"
     "Synchronize data cache")
    ("SYNCDI" "SYNCD $X, $Y, Z"
     "Synchronize data cache (immediate)")
    ("SYNCID" "SYNCID $X, $Y, $Z"
     "Synchronize instruction and data cache")
    ("SYNCIDI" "SYNCID $X, $Y, Z"
     "Synchronize instruction and data cache (immediate)")
    ("debug" "debug \"text\""
     "checksmix preprocessor line: print text and a newline to StdOut, preserving registers"))
  "One row per opcode and directive: (NAME SYNTAX DESCRIPTION).
NAME is checksmix's opcode name, SYNTAX the source spellings separated
by \" / \".")

(defconst mmix--instruction-help
  (let ((table (make-hash-table :test #'equal))
        (case-fold-search nil))
    (dolist (row mmix-instruction-reference)
      (pcase-let* ((`(,name ,syntax ,description) row)
                   (entry (cons syntax description))
                   (keys (list name)))
        (dolist (form (split-string syntax " / " t))
          (when (string-match "\\(?:^\\| \\)\\([0-9]*[A-Z][A-Z0-9]*\\)\\_>" form)
            (push (match-string 1 form) keys)))
        (dolist (key (delete-dups keys))
          (puthash key (append (gethash key table) (list entry)) table))))
    table)
  "Map from a mnemonic to its (SYNTAX . DESCRIPTION) entries.
A row is filed under its NAME and under every mnemonic its SYNTAX
spells, so ADD lists both the register and the immediate form.")

(defun mmix-instruction-help (word)
  "Return the (SYNTAX . DESCRIPTION) entries documenting WORD."
  (when (mmix--keyword-kind word)
    (gethash (if (string= word mmix-debug-directive)
                 word
               (let ((name (string-remove-prefix "." (upcase word))))
                 (cond
                  ((string= name "QUAD") "OCTA")
                  ;; The immediate form of 2ADDU is named ADDU2I.
                  ((string-match "\\`\\([0-9]+\\)ADDUI\\'" name)
                   (format "ADDU%sI" (match-string 1 name)))
                  (t name))))
             mmix--instruction-help)))

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

(defun mmix-run ()
  "Assemble and run the current file with `checksmix run' under `compile'."
  (interactive)
  (unless buffer-file-name
    (user-error "Buffer is not visiting a file"))
  (save-buffer)
  (compile (format "%s run %s" mmix-checksmix-program
                   (shell-quote-argument buffer-file-name))))

;;;; Mode

(defun mmix-imenu-index ()
  "Return an imenu index of the buffer's labels."
  (let (index)
    (save-excursion
      (goto-char (point-min))
      (while (not (eobp))
        (pcase (mmix--line-fields)
          (`(,beg ,end . ,_)
           (when beg
             (push (cons (string-trim-right
                          (buffer-substring-no-properties beg end) ":")
                         (copy-marker beg))
                   index))))
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
  (setq-local comment-start-skip "[%;]+[ \t]*")
  (setq-local comment-column 40)
  (setq-local syntax-propertize-function #'mmix--syntax-propertize)
  (setq-local font-lock-defaults '(mmix-font-lock-keywords nil nil))
  (setq-local indent-line-function #'mmix-indent-line)
  (setq-local indent-tabs-mode t)
  (setq-local tab-width 8)
  (setq-local imenu-create-index-function #'mmix-imenu-index)
  (add-hook 'eldoc-documentation-functions #'mmix-eldoc-function nil t)
  (when buffer-file-name
    (setq-local compile-command
                (format "%s run %s" mmix-checksmix-program
                        (shell-quote-argument buffer-file-name)))))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.mms\\'" . mmix-mode))

(provide 'mmix-mode)
;;; mmix-mode.el ends here
