;;; mmix-mode-test.el --- ert tests for mmix-mode.el -*- lexical-binding: t; -*-

;; Run from the repository root:
;;
;;   emacs --batch -L contrib -l contrib/mmix-mode-test.el \
;;     -f ert-run-tests-batch-and-exit
;;
;; The keyword and predefined-symbol tests read src/mmixal.pest and
;; src/mmixal.rs, so a mnemonic or symbol added to the assembler without
;; the mode turns them red.  The mode itself reads no file.

;;; Code:

(require 'ert)
(require 'mmix-mode)

(defconst mmix-test--root
  (expand-file-name ".." (file-name-directory
                          (or load-file-name buffer-file-name)))
  "The checksmix repository root.")

(defun mmix-test--file-string (relative)
  "Return the contents of RELATIVE under the repository root."
  (with-temp-buffer
    (insert-file-contents (expand-file-name relative mmix-test--root))
    (buffer-string)))

(defun mmix-test--matches (regexp string group)
  "Return every GROUP match of REGEXP in STRING, sorted and deduplicated."
  (let ((case-fold-search nil) (start 0) found)
    (while (string-match regexp string start)
      (push (match-string group string) found)
      (setq start (match-end 0)))
    (sort (delete-dups found) #'string<)))

(defmacro mmix-test--with-buffer (source &rest body)
  "Run BODY in a fontified `mmix-mode' buffer holding SOURCE."
  (declare (indent 1))
  `(with-temp-buffer
     (insert ,source)
     (mmix-mode)
     (font-lock-ensure)
     (goto-char (point-min))
     ,@body))

(defun mmix-test--face-at (needle &optional occurrence)
  "Return the face at the start of the OCCURRENCE-th NEEDLE in the buffer."
  (goto-char (point-min))
  (let ((case-fold-search nil))
    (dotimes (_ (or occurrence 1))
      (search-forward needle)))
  (get-text-property (match-beginning 0) 'face))

;;;; Agreement with the assembler

(ert-deftest mmix-keywords-match-the-grammar ()
  "Every keyword the grammar accepts is a mode keyword, and no other.
INCLUDE is a preprocessor stage outside the grammar."
  (should
   (equal (mmix-test--matches "\\^\"\\([.A-Z0-9]+\\)\""
                              (mmix-test--file-string "src/mmixal.pest") 1)
          (sort (seq-difference (append mmix-instructions mmix-directives)
                                '("INCLUDE" ".INCLUDE"))
                #'string<))))

(ert-deftest mmix-predefined-symbols-match-the-assembler ()
  "The mode's predefined symbols are exactly those the assembler seeds."
  (let* ((source (mmix-test--file-string "src/mmixal.rs"))
         (beg (string-search "pub fn new(source: &str" source))
         (end (string-search "Self::preprocess_debug(source" source beg)))
    (should (and beg end))
    (should
     (equal (mmix-test--matches "\"\\([A-Za-z_]+\\)\""
                                (substring source beg end) 1)
            (sort (append (mapcar #'car mmix-special-registers)
                          (mapcar #'car mmix-predefined-constants))
                  #'string<)))))

(ert-deftest mmix-every-keyword-has-help ()
  "The built-in reference documents every mnemonic and directive."
  (should (null (seq-remove #'mmix-instruction-help
                            (append mmix-instructions mmix-directives
                                    (list mmix-debug-directive))))))

;;;; Help

(ert-deftest mmix-help-covers-both-operand-forms ()
  "A base mnemonic lists its register and immediate rows."
  (let ((help (mmix-instruction-help "add")))
    (should (member '("ADD $X, $Y, $Z" . "Add signed (sets overflow)") help))
    (should (member '("ADD $X, $Y, Z" . "Add signed immediate") help))))

(ert-deftest mmix-help-reads-spelled-mnemonics-and-aliases ()
  "2ADDU, 16ADDUI, .BYTE and QUAD resolve to their rows."
  (should (string-prefix-p "2ADDU" (caar (mmix-instruction-help "2ADDU"))))
  (should (equal (mmix-instruction-help "16ADDUI")
                 '(("16ADDU $X, $Y, Z" . "$X = 16*$Y + Z unsigned"))))
  (should (string-prefix-p "BYTE" (caar (mmix-instruction-help ".byte"))))
  (should (equal (mmix-instruction-help "QUAD")
                 (mmix-instruction-help "OCTA"))))

(ert-deftest mmix-help-ignores-non-keywords ()
  "Only a keyword has help; debug is case-sensitive."
  (should-not (mmix-instruction-help "Main"))
  (should-not (mmix-instruction-help "DEBUG"))
  (should (mmix-instruction-help "debug")))

(ert-deftest mmix-help-describes-predefined-symbols ()
  "Special registers and TRAP codes are documented, case-sensitively."
  (should (equal (mmix-symbol-help "rJ")
                 "rJ: special register 4, return-jump register"))
  (should (string-match-p "TRAP function 7: Write a null-terminated string"
                          (mmix-symbol-help ":Fputs")))
  (should-not (mmix-symbol-help "rj")))

(ert-deftest mmix-help-works-from-a-lone-copy ()
  "A copy of the mode alone in a directory, loaded by a fresh Emacs, has help."
  (let ((dir (make-temp-file "mmix-mode-" t)))
    (unwind-protect
        (progn
          (copy-file (locate-library "mmix-mode.el" t) (file-name-as-directory dir))
          (with-temp-buffer
            (should
             (zerop
              (call-process
               (expand-file-name invocation-name invocation-directory)
               nil t nil "--batch" "-Q" "-L" dir "-l" "mmix-mode"
               "--eval" "(princ (car (car (mmix-instruction-help \"PUSHJ\"))))")))
            (should (string-match-p "PUSHJ \\$X, addr" (buffer-string)))))
      (delete-directory dir t))))

(ert-deftest mmix-eldoc-documents-the-line-instruction ()
  "eldoc reports the instruction anywhere on its line."
  (mmix-test--with-buffer "Main\tSUB\t$1,$2,$3\t% subtract\n"
    (search-forward "$3")
    (let (reported)
      (mmix-eldoc-function (lambda (text &rest _) (setq reported text)))
      (should (string-match-p "Subtract signed" reported)))))

;;;; Statement fields and font lock

(ert-deftest mmix-first-word-is-a-label-unless-it-is-an-operation ()
  "Labels need not start in column 0, and a mnemonic may name a label."
  (mmix-test--with-buffer
      (concat "Main\tSETL\t$0,1\n"
              "  Loop ADD $1,$1,1\n"
              "\tADD\t$1,$1,1\n"
              "Add\tADD\t$1,$1,1\n"
              "Done\n"
              "Set\n"
              "\tHALT\n"
              "Next:SUB $1,$1,1\n"
              ":Glob\tSWYM\n"
              "\tFOO\t$1\n")
    (should (eq (mmix-test--face-at "Main") 'font-lock-function-name-face))
    (should (eq (mmix-test--face-at "Loop") 'font-lock-function-name-face))
    (should (eq (mmix-test--face-at "ADD" 1) 'font-lock-keyword-face))
    (should (eq (mmix-test--face-at "ADD" 2) 'font-lock-keyword-face))
    (should (eq (mmix-test--face-at "Add") 'font-lock-function-name-face))
    (should (eq (mmix-test--face-at "ADD" 3) 'font-lock-keyword-face))
    (should (eq (mmix-test--face-at "Done") 'font-lock-function-name-face))
    (should (eq (mmix-test--face-at "Set\n") 'font-lock-function-name-face))
    (should (eq (mmix-test--face-at "HALT") 'font-lock-keyword-face))
    (should (eq (mmix-test--face-at "Next:") 'font-lock-function-name-face))
    (should (eq (mmix-test--face-at "SUB") 'font-lock-keyword-face))
    (should (eq (mmix-test--face-at ":Glob") 'font-lock-function-name-face))
    (should-not (mmix-test--face-at "FOO"))))

(ert-deftest mmix-single-operand-statements-take-a-keyword-named-operand ()
  "A lone keyword-named word after a single-operand keyword is its operand.
checksmix assembles `JMP ADD' as a jump to ADD and `Loc HALT' as LOC, but
reads `Set HALT' as the label Set on a HALT."
  (mmix-test--with-buffer
      (concat "\tJMP\tADD\n"
              "Loc\tHALT\n"
              "\tPREFIX\tSET\n"
              "Set\tHALT\n"
              "\tJMP\tADD $1\n"
              "\tINCLUDE\tADD\n")
    (should (eq (mmix-test--face-at "JMP") 'font-lock-keyword-face))
    (should-not (mmix-test--face-at "ADD\n"))
    (should (eq (mmix-test--face-at "Loc") 'font-lock-preprocessor-face))
    (should (eq (mmix-test--face-at "PREFIX") 'font-lock-preprocessor-face))
    (should (eq (mmix-test--face-at "Set\t") 'font-lock-function-name-face))
    (should (eq (mmix-test--face-at "HALT" 2) 'font-lock-keyword-face))
    (should (eq (mmix-test--face-at "JMP" 2) 'font-lock-function-name-face))
    (should (eq (mmix-test--face-at "ADD $1") 'font-lock-keyword-face))
    (should (eq (mmix-test--face-at "INCLUDE") 'font-lock-preprocessor-face))
    (should-not (mmix-test--face-at "ADD\n" 2)))
  (should (equal (mmix-test--indent "\tJMP\tADD\n\tLOC\tGREG\n")
                 "\tJMP\tADD\n\tLOC\tGREG\n")))

(ert-deftest mmix-debug-is-a-directive-only-where-checksmix-expands-it ()
  "Nothing may follow the text of a debug line, not even a comment."
  (mmix-test--with-buffer
      "Main\tdebug \"ok\"  \n\tdebug \"bad\" % note\n"
    (should (eq (mmix-test--face-at "debug") 'font-lock-preprocessor-face))
    (should-not (mmix-test--face-at "debug" 2))))

(ert-deftest mmix-keywords-are-case-insensitive-and-symbols-are-not ()
  "halt is the instruction; Halt in an operand is the TRAP constant."
  (mmix-test--with-buffer "\thalt\n\tTRAP\t0,Halt,0\n\tGET\t$1,rJ\n\tGET\t$1,rj\n"
    (should (eq (mmix-test--face-at "halt") 'font-lock-keyword-face))
    (should (eq (mmix-test--face-at "Halt") 'font-lock-constant-face))
    (should (eq (mmix-test--face-at "rJ") 'font-lock-builtin-face))
    (should-not (mmix-test--face-at "rj"))))

(ert-deftest mmix-directives-and-definitions ()
  "Directives, dotted and QUAD spellings, INCLUDE and debug are directives.
A name bound by IS or GREG is a variable, not a label."
  (mmix-test--with-buffer
      (concat "Five\tIS\t5\n"
              "Sp\tGREG\t@\n"
              "\t.BYTE\t1\n"
              "\tquad\t2\n"
              "\tINCLUDE\tlib.mms\n"
              "Main\tdebug \"hi\"\n"
              "\t2ADDU\t$1,$2,$3\n")
    (should (eq (mmix-test--face-at "Five") 'font-lock-variable-name-face))
    (should (eq (mmix-test--face-at "IS") 'font-lock-preprocessor-face))
    (should (eq (mmix-test--face-at "Sp") 'font-lock-variable-name-face))
    (should (eq (mmix-test--face-at ".BYTE") 'font-lock-preprocessor-face))
    (should (eq (mmix-test--face-at "quad") 'font-lock-preprocessor-face))
    (should (eq (mmix-test--face-at "INCLUDE") 'font-lock-preprocessor-face))
    (should (eq (mmix-test--face-at "debug") 'font-lock-preprocessor-face))
    (should (eq (mmix-test--face-at "Main") 'font-lock-function-name-face))
    (should (eq (mmix-test--face-at "2ADDU") 'font-lock-keyword-face))
    (should (eq (mmix-test--face-at "@") 'font-lock-number-face))))

(ert-deftest mmix-comments-strings-and-character-literals ()
  "Both comment characters work; neither opens a comment inside a literal."
  (mmix-test--with-buffer
      (concat "\tSETL\t$0,1\t% percent comment\n"
              "\tSETL\t$0,1\t; semicolon comment\n"
              "Text\tBYTE\t\"50% ; off\\\",'%',';','\\'',0\n"
              "\tSETL\t$1,2\n")
    (should (eq (mmix-test--face-at "percent") 'font-lock-comment-face))
    (should (eq (mmix-test--face-at "semicolon") 'font-lock-comment-face))
    (should (eq (mmix-test--face-at "50%") 'font-lock-string-face))
    (should (eq (mmix-test--face-at "'%'") 'font-lock-string-face))
    (should (eq (mmix-test--face-at "';'") 'font-lock-string-face))
    (should (eq (mmix-test--face-at "'\\''") 'font-lock-string-face))
    (should (eq (mmix-test--face-at ",0") nil))
    (should (eq (mmix-test--face-at "$1,2") 'font-lock-variable-name-face))))

(ert-deftest mmix-numbers ()
  "Decimal, #hex, 0x hex, octal and negative literals are numbers.
Digits inside a register, a symbol or a mnemonic are not."
  (mmix-test--with-buffer "\tSET\t$12,#FF\n\tSET\t$1,-0x1f\n\tSET\t$1,017\n\tSET\tX2,Y\n"
    (should (eq (mmix-test--face-at "#FF") 'font-lock-number-face))
    (should (eq (mmix-test--face-at "-0x1f") 'font-lock-number-face))
    (should (eq (mmix-test--face-at "017") 'font-lock-number-face))
    (should (eq (mmix-test--face-at "12") 'font-lock-variable-name-face))
    (should-not (mmix-test--face-at "2,Y"))))

;;;; Symbol references

(ert-deftest mmix-references-take-the-face-of-their-definition ()
  "A label use is a function name and an IS or GREG name a variable.
Forward references count; undefined names, near-misses in case or
colon, hex digits, comments and strings do not."
  (mmix-test--with-buffer
      (concat "MOD\tIS\t100\n"
              "Sp\tGREG\t@\n"
              "Main\tSETI\t$2,MOD\n"
              "\tPUSHJ\t$0,RemEuclid\n"
              "\tLDO\t$1,Sp,0\n"
              "\tBNZ\t$3,Undefined\n"
              "\tBNZ\t$3,remeuclid\n"
              "\tBNZ\t$3,:RemEuclid\n"
              "\tSET\t$1,#FF\n"
              "\tSET\t$1,1\t% RemEuclid in a comment\n"
              "\tBYTE\t\"RemEuclid\",0\n"
              "RemEuclid\tPOP\t1,0\n"
              "FF\tSWYM\n"
              ":Glob\tSWYM\n"
              "\tJMP\t:Glob\n"
              "\tJMP\tGlob\n")
    (should (eq (mmix-test--face-at "MOD\n") 'font-lock-variable-name-face))
    (should (eq (mmix-test--face-at "RemEuclid\n") 'font-lock-function-name-face))
    (should (eq (mmix-test--face-at "Sp,") 'font-lock-variable-name-face))
    (should-not (mmix-test--face-at "Undefined"))
    (should-not (mmix-test--face-at "remeuclid"))
    (should-not (mmix-test--face-at ":RemEuclid"))
    (should-not (mmix-test--face-at "RemEuclid\n" 2))
    (should (eq (mmix-test--face-at "FF\n") 'font-lock-number-face))
    (should (eq (mmix-test--face-at "RemEuclid in") 'font-lock-comment-face))
    (should (eq (mmix-test--face-at "RemEuclid\"") 'font-lock-string-face))
    (should (eq (mmix-test--face-at ":Glob\n") 'font-lock-function-name-face))
    (should-not (mmix-test--face-at "\tGlob\n"))))

(defun mmix-test--run-rescan ()
  "Run the pending definitions rescan now; return how often it flushed."
  (let* ((timer mmix--definitions-timer)
         (flushes 0)
         (count (lambda (&rest _) (setq flushes (1+ flushes)))))
    (should (timerp timer))
    (cancel-timer timer)
    (advice-add 'font-lock-flush :before count)
    (unwind-protect
        (apply (timer--function timer) (timer--args timer))
      (advice-remove 'font-lock-flush count))
    flushes))

(ert-deftest mmix-references-follow-edits ()
  "An edit schedules a rescan rather than scanning during fontification.
The rescan picks up a new definition and refontifies, and does not
refontify when the defined names are unchanged."
  (mmix-test--with-buffer "\tJMP\tLater\n"
    (should-not (mmix-test--face-at "Later"))
    (goto-char (point-max))
    (insert "Later\tSWYM\n")
    (should-not (gethash "Later" (mmix--definitions)))
    (should (= (mmix-test--run-rescan) 1))
    (should-not mmix--definitions-timer)
    (should (eq (gethash "Later" (mmix--definitions)) 'label))
    (font-lock-ensure)
    (should (eq (mmix-test--face-at "Later") 'font-lock-function-name-face))
    (goto-char (point-max))
    (insert "% no new names\n")
    (mmix--definitions)
    (should (= (mmix-test--run-rescan) 0))))

(ert-deftest mmix-rescan-does-nothing-after-leaving-the-mode ()
  "A rescan scheduled in `mmix-mode' does not run in the buffer's next mode."
  (mmix-test--with-buffer "Foo\tSWYM\n"
    (insert "Bar\tSWYM\n")
    (mmix--definitions)
    (let ((timer mmix--definitions-timer))
      (should (timerp timer))
      (cancel-timer timer)
      (fundamental-mode)
      (apply (timer--function timer) (timer--args timer))
      (should-not (local-variable-p 'mmix--definitions-cache)))))

(ert-deftest mmix-a-colon-label-before-is-defines-nothing ()
  "checksmix rejects `Five: IS 5', so Five is neither highlighted nor
tracked; a colon label before GREG or LOC is accepted."
  (mmix-test--with-buffer
      "Five: IS 5\nSp:\tGREG\t@\n\tSETL\t$0,Five\n\tLDO\t$1,Sp,0\n"
    (should-not (mmix-test--face-at "Five:"))
    (should-not (mmix-test--face-at "Five\n"))
    (should-not (gethash "Five" (mmix--definitions)))
    (should (eq (mmix-test--face-at "Sp:") 'font-lock-variable-name-face))
    (should (eq (mmix-test--face-at "Sp,") 'font-lock-variable-name-face))))

(ert-deftest mmix-definition-tables-compare-names-and-kinds ()
  "Tables differ when a name is added, removed or changes kind."
  (let ((a (make-hash-table :test #'equal))
        (b (make-hash-table :test #'equal)))
    (puthash "Foo" 'label a)
    (should-not (mmix--same-names-p a b))
    (puthash "Foo" 'value b)
    (should-not (mmix--same-names-p a b))
    (puthash "Foo" 'label b)
    (should (mmix--same-names-p a b))))

;;;; Indentation

(defun mmix-test--indent (source)
  "Return SOURCE after `indent-region' in `mmix-mode'."
  (with-temp-buffer
    (insert source)
    (mmix-mode)
    (indent-region (point-min) (point-max))
    (buffer-string)))

(ert-deftest mmix-indent-aligns-fields ()
  "Labels go to column 0, operations to 8, operands to 16."
  (should (equal (mmix-test--indent
                  (concat "   Main SETL $0,1 % one\n"
                          "ADD $1,$1,1\n"
                          "LongLabelName PUSHJ $0,Fibonacci\n"
                          "Done\n"
                          "% header\n"
                          "      ; aside\n"))
                 (concat "Main\tSETL\t$0,1 % one\n"
                         "\tADD\t$1,$1,1\n"
                         "LongLabelName PUSHJ $0,Fibonacci\n"
                         "Done\n"
                         "% header\n"
                         "\t; aside\n"))))

(ert-deftest mmix-indent-leaves-a-word-being-typed-in-the-operation-field ()
  "An indented lone word is an operation in progress, not a label."
  (should (equal (mmix-test--indent "  SETL\n") "\tSETL\n")))

;;;; checksmix

(ert-deftest mmix-compile-command-runs-checksmix ()
  "A visited file's compile command is `checksmix run FILE'."
  (with-temp-buffer
    (setq buffer-file-name "/tmp/prog.mms")
    (mmix-mode)
    (should (equal compile-command "checksmix run /tmp/prog.mms"))
    (set-buffer-modified-p nil)
    (setq buffer-file-name nil)))

(ert-deftest mmix-imenu-lists-labels ()
  "imenu indexes labels and IS names, without trailing colons."
  (mmix-test--with-buffer "Main\tSETL\t$0,1\n\tADD\t$1,$1,1\nLoop:\n"
    (should (equal (mapcar #'car (mmix-imenu-index)) '("Main" "Loop")))))

(ert-deftest mmix-files-open-in-mmix-mode ()
  "A .mms file selects the mode."
  (should (eq (assoc-default "prog.mms" auto-mode-alist #'string-match)
              'mmix-mode)))

;;; mmix-mode-test.el ends here
