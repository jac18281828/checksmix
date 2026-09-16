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
;; Instruction help is read from checksmix's own MMIX.md, located one
;; directory above this file.  `eldoc' shows the syntax and description of
;; the instruction on the current line, or of the predefined symbol at
;; point; C-c C-d describes any instruction in a help buffer.  Without
;; MMIX.md the mode still highlights and indents; only instruction help is
;; unavailable.
;;
;; C-c C-c runs the buffer's file through `checksmix run' under `compile'.
;;
;; To install, put this directory on `load-path' and require it:
;;
;;   (add-to-list 'load-path "/path/to/checksmix/contrib")
;;   (require 'mmix-mode)

;;; Code:

(require 'compile)
(require 'eldoc)
(require 'help-mode)

(defgroup mmix nil
  "Editing checksmix MMIXAL source."
  :group 'languages)

(defcustom mmix-reference-file
  (when load-file-name
    (expand-file-name "../MMIX.md" (file-name-directory load-file-name)))
  "checksmix's MMIX.md, the source of instruction help."
  :type '(choice (const :tag "None" nil) file))

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
    ("Halt" . "TRAP function code") ("Trip" . "TRAP function code")
    ("Fopen" . "TRAP function code") ("Fclose" . "TRAP function code")
    ("Fread" . "TRAP function code") ("Fgets" . "TRAP function code")
    ("Fgetws" . "TRAP function code") ("Fwrite" . "TRAP function code")
    ("Fputs" . "TRAP function code") ("Fputc" . "TRAP function code")
    ("Fputws" . "TRAP function code") ("Fseek" . "TRAP function code")
    ("Ftell" . "TRAP function code") ("Time" . "TRAP function code")
    ("ROUND_CURRENT" . "rounding-mode override 0: use rA's mode")
    ("ROUND_OFF" . "rounding-mode override 1: toward zero")
    ("ROUND_UP" . "rounding-mode override 2: toward +infinity")
    ("ROUND_DOWN" . "rounding-mode override 3: toward -infinity")
    ("ROUND_NEAR" . "rounding-mode override 4: to nearest, ties to even"))
  "Constant symbols checksmix predefines, other than special registers.
Each is (NAME . MEANING).  A TRAP function code's fuller description
comes from MMIX.md's TRAP table.")

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

(defun mmix--line-fields ()
  "Return the statement fields of the current line, or nil.
The value is (LABEL-BEG LABEL-END OP-BEG OP-END OPERAND-BEG); a field
absent from the line is nil.

checksmix tries a line as a bare statement before it tries a label, so
the first word is the operation when it cannot be a label, or when it is
a mnemonic or directive that is not followed by another and does not
stand alone (HALT and SWYM may), or when operands follow it directly.
Otherwise it is a label and the next word, if any, is the operation."
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
               (operands-follow (and (not end2) (mmix--operands-at-point-p)))
               (op-first
                (or (not (string-match-p mmix--label-regexp word1))
                    (and (mmix--keyword-kind word1)
                         (not (and word2 (mmix--keyword-kind word2)))
                         (or word2 operands-follow
                             (member (upcase word1) '("HALT" "SWYM"))))
                    operands-follow))
               (op-beg (if op-first beg1 (and end2 beg2)))
               (op-end (if op-first end1 end2)))
          (when op-end
            (goto-char op-end)
            (skip-chars-forward " \t"))
          (list (unless op-first beg1) (unless op-first end1) op-beg op-end
                (and op-end (mmix--operands-at-point-p) (point))))))))

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

(defun mmix--match-statement (limit)
  "Find the next statement line before LIMIT and set its match data.
Group 1 is a label, group 2 an instruction, group 3 a directive."
  (let (found)
    (while (and (not found) (< (point) limit) (not (eobp)))
      (let ((fields (mmix--line-fields)))
        (forward-line 1)
        (pcase fields
          (`(,label-beg ,label-end ,op-beg ,op-end ,_)
           (let ((kind (and op-beg (mmix--keyword-kind
                                    (buffer-substring-no-properties
                                     op-beg op-end)))))
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
  (if (member (upcase (or (match-string-no-properties 3) "")) '("IS" "GREG"))
      'font-lock-variable-name-face
    'font-lock-function-name-face))

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
    ("@" . 'font-lock-number-face))
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

(defvar mmix--reference-cache nil
  "(FILE MTIME INSTRUCTIONS SYMBOLS) parsed from `mmix-reference-file'.")

(defun mmix--strip-markdown (text)
  "Return TEXT with Markdown code and emphasis markers removed."
  (replace-regexp-in-string
   "\\*\\([^*]+\\)\\*" "\\1" (string-replace "`" "" text)))

(defun mmix--section-rows (heading)
  "Return the table rows under the `## HEADING' section of this buffer.
Each row is (NAME SYNTAX DESCRIPTION), where SYNTAX is the second cell
and DESCRIPTION the rest of the row, which may itself contain `|'."
  (goto-char (point-min))
  (let (rows)
    (when (re-search-forward (concat "^## " (regexp-quote heading) "$") nil t)
      (let ((end (save-excursion
                   (if (re-search-forward "^## " nil t)
                       (match-beginning 0)
                     (point-max)))))
        (while (re-search-forward
                "^| `\\([^`]+\\)` | \\(.*?\\) | \\(.*\\) |$" end t)
          (push (list (match-string-no-properties 1)
                      (match-string-no-properties 2)
                      (match-string-no-properties 3))
                rows))))
    (nreverse rows)))

(defun mmix--parse-reference (file)
  "Parse checksmix's MMIX.md FILE into (INSTRUCTIONS . SYMBOLS).
INSTRUCTIONS maps an upper-case mnemonic to a list of (SYNTAX
. DESCRIPTION), keyed under the table's mnemonic and under every
mnemonic its syntax column spells.  SYMBOLS maps a TRAP function name to
its description."
  (let ((instructions (make-hash-table :test #'equal))
        (symbols (make-hash-table :test #'equal)))
    (with-temp-buffer
      (insert-file-contents file)
      (dolist (row (append (mmix--section-rows "Assembler directives")
                           (mmix--section-rows "Instruction table")))
        (pcase-let* ((`(,name ,syntax ,description) row)
                     (entry (cons (mmix--strip-markdown syntax)
                                  (mmix--strip-markdown description)))
                     (keys (list name)))
          (dolist (form (split-string syntax "` / `" t "[` ]+"))
            (let ((case-fold-search nil))
              (when (string-match "\\(?:^\\| \\)\\([0-9]*[A-Z][A-Z0-9]*\\)\\_>"
                                  form)
                (push (match-string 1 form) keys))))
          (dolist (key (delete-dups keys))
            (unless (member entry (gethash key instructions))
              (puthash key (append (gethash key instructions) (list entry))
                       instructions)))))
      (dolist (row (mmix--section-rows "TRAP interface"))
        (pcase-let ((`(,name ,code ,description) row))
          (puthash name (mmix--strip-markdown
                         (format "TRAP function %s: %s" code description))
                   symbols))))
    (cons instructions symbols)))

(defun mmix--reference ()
  "Return the parsed reference as (INSTRUCTIONS . SYMBOLS), or nil."
  (when (and mmix-reference-file (file-readable-p mmix-reference-file))
    (let ((mtime (file-attribute-modification-time
                  (file-attributes mmix-reference-file))))
      (unless (and (equal (nth 0 mmix--reference-cache) mmix-reference-file)
                   (equal (nth 1 mmix--reference-cache) mtime))
        (let ((parsed (mmix--parse-reference mmix-reference-file)))
          (setq mmix--reference-cache
                (list mmix-reference-file mtime (car parsed) (cdr parsed)))))
      (cons (nth 2 mmix--reference-cache) (nth 3 mmix--reference-cache)))))

(defun mmix-instruction-help (word)
  "Return the (SYNTAX . DESCRIPTION) entries documenting WORD."
  (cond
   ((string= word mmix-debug-directive)
    '(("debug \"text\""
       . "checksmix preprocessor line: print text and a newline to StdOut, preserving registers")))
   ((mmix--keyword-kind word)
    (let* ((name (string-remove-prefix "." (upcase word)))
           (name (cond
                  ((string= name "QUAD") "OCTA")
                  ;; MMIX.md spells the immediate form of 2ADDU as ADDU2I.
                  ((string-match "\\`\\([0-9]+\\)ADDUI\\'" name)
                   (format "ADDU%sI" (match-string 1 name)))
                  (t name))))
      (when-let* ((reference (mmix--reference)))
        (gethash name (car reference)))))))

(defun mmix-symbol-help (symbol)
  "Return a one-line description of predefined SYMBOL, or nil."
  (let ((name (string-remove-prefix ":" symbol)))
    (if-let* ((register (assoc name mmix-special-registers)))
        (format "%s: special register %d, %s"
                name (nth 1 register) (nth 2 register))
      (when-let* ((constant (assoc name mmix-predefined-constants)))
        (format "%s: %s" name
                (or (when-let* ((reference (mmix--reference)))
                      (gethash name (cdr reference)))
                    (cdr constant)))))))

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
      (user-error "No help for %s; is `mmix-reference-file' set?" word))
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
