;;; mmixdb.el --- gud mode for the mmixdb MMIX debugger -*- lexical-binding: t; -*-

;; Minimal GUD integration for `mmixdb'.  Provides `M-x mmixdb', which runs the
;; debugger under `gud-mode' with the standard GUD key bindings mapped onto
;; mmixdb's long command words.
;;
;; mmixdb emits gdb's `--fullname' stop marker (`\032\032 FILE:LINE:...')
;; whenever INSIDE_EMACS is set, which comint sets for every subprocess, so no
;; flag is needed and gud's own gdb marker filter parses the marker unchanged.
;;
;; To install, put this directory on `load-path' and require it:
;;
;;   (add-to-list 'load-path "/path/to/checksmix/contrib")
;;   (require 'mmixdb)
;;
;; or autoload it instead:
;;
;;   (autoload 'mmixdb "mmixdb" "Run mmixdb under gud-mode." t)

;;; Code:

(require 'gud)

(defvaralias 'gud-mmixdb-marker-regexp 'gud-gdb-marker-regexp
  "Regexp matching mmixdb's stop marker.
mmixdb's marker has gdb's `--fullname' shape, so this is gud's own gdb
regexp rather than a copy of it.")

(defalias 'gud-mmixdb-marker-filter #'gud-gdb-marker-filter
  "Extract the current source line from mmixdb output.
mmixdb's marker has gdb's `--fullname' shape, so this is gud's own gdb
filter rather than a reimplementation of it.")

;;;###autoload
(defun mmixdb (command-line)
  "Run mmixdb on COMMAND-LINE under `gud-mode'."
  (interactive (list (gud-query-cmdline 'mmixdb)))
  (gud-common-init command-line nil #'gud-mmixdb-marker-filter)
  (set (make-local-variable 'gud-minor-mode) 'mmixdb)
  (gud-def gud-step   "step"     "\C-s" "Step one source line, entering calls.")
  (gud-def gud-next   "next"     "\C-n" "Step one source line, stepping over calls.")
  (gud-def gud-stepi  "stepi"    "\C-i" "Step one instruction, entering calls.")
  (gud-def gud-cont   "continue" "\C-r" "Continue until breakpoint or halt.")
  (gud-def gud-break  "break %l" "\C-b" "Set breakpoint at current line.")
  (gud-def gud-print  "print %e" "\C-p" "Print value at point.")
  (run-hooks 'mmixdb-mode-hook))

(provide 'mmixdb)
;;; mmixdb.el ends here
