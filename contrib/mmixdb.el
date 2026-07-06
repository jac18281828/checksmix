;;; mmixdb.el --- gud mode for the mmixdb MMIX debugger -*- lexical-binding: t; -*-

;; Minimal GUD integration for `mmixdb'.  Provides `M-x mmixdb', which starts
;; the debugger with `--fullname' and parses its Emacs marker
;; (`\x1a\x1a FILE:LINE:...') to jump `gud' to the current source line, plus
;; the standard gdb-style key bindings mapped onto mmixdb's long command
;; words.

;;; Code:

(require 'gud)

(defvar gud-mmixdb-marker-regexp
  "\032\032\\(.*\\):\\([0-9]+\\):.*\n"
  "Regexp matching mmixdb's `--fullname' stop marker.")

(defun gud-mmixdb-marker-filter (string)
  "Filter mmixdb output STRING, extracting the current source line."
  (setq gud-marker-acc (concat (or gud-marker-acc "") string))
  (let (output)
    (when (string-match gud-mmixdb-marker-regexp gud-marker-acc)
      (setq gud-last-frame
            (cons (match-string 1 gud-marker-acc)
                  (string-to-number (match-string 2 gud-marker-acc))))
      (setq output (substring gud-marker-acc 0 (match-beginning 0)))
      (setq gud-marker-acc (substring gud-marker-acc (match-end 0))))
    (or output "")))

;;;###autoload
(defun mmixdb (command-line)
  "Run mmixdb on COMMAND-LINE under `gud-mode'."
  (interactive (list (gud-query-cmdline 'mmixdb)))
  (gud-common-init command-line nil 'gud-mmixdb-marker-filter)
  (set (make-local-variable 'gud-minor-mode) 'mmixdb)
  (gud-def gud-step   "step"     "\C-s" "Step one instruction, entering calls.")
  (gud-def gud-next   "next"     "\C-n" "Step one instruction, stepping over calls.")
  (gud-def gud-cont   "continue" "\C-r" "Continue until breakpoint or halt.")
  (gud-def gud-break  "break %l" "\C-b" "Set breakpoint at current line.")
  (gud-def gud-print  "print %e" "\C-p" "Print value at point.")
  (run-hooks 'mmixdb-mode-hook))

(provide 'mmixdb)
;;; mmixdb.el ends here
