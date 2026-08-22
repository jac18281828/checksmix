;;; mmixdb-test.el --- ert tests for mmixdb.el -*- lexical-binding: t; -*-

;; Run from the repository root:
;;
;;   emacs --batch -l gud -l contrib/mmixdb.el -l contrib/mmixdb-test.el \
;;     -f ert-run-tests-batch-and-exit
;;
;; These tests resolve the filter and the regexp through the names
;; `contrib/mmixdb.el' binds, so reinstating a hand-written filter or regexp
;; turns them red.

;;; Code:

(require 'ert)
(require 'gud)
(require 'mmixdb)

(defconst mmixdb-test--stop
  (concat "\032\032/w/example.mms:8:0:beg:0x100\n"
          "/w/example.mms:8\tMain    SETI    $1,7\n"
          "(mmixdb) ")
  "One mmixdb stop, recorded from a live session under Emacs.
Emacs disables ECHO and ONLCR on a child pty, so the stream carries the
marker, the location line and the prompt separated by bare newlines.  The
absolute path is shortened for readability.")

(defmacro mmixdb-test--with-marker-state (&rest body)
  "Run BODY with gud's marker accumulator and frame freshly bound."
  (declare (indent 0))
  `(let ((gud-marker-acc "")
         (gud-last-frame nil))
     ,@body))

(ert-deftest mmixdb-filter-returns-everything-but-the-marker ()
  "The filter passes the debugger's own output through to the buffer."
  (mmixdb-test--with-marker-state
    (let ((output (gud-mmixdb-marker-filter mmixdb-test--stop)))
      (should (string-match-p "example\\.mms:8\tMain    SETI" output))
      (should (string-match-p "(mmixdb) " output))
      (should-not (string-match-p "\032\032" output)))))

(ert-deftest mmixdb-filter-sets-the-frame-to-a-bare-path-and-a-line ()
  "The frame's car is the path alone; its cdr is the line as a number."
  (mmixdb-test--with-marker-state
    (gud-mmixdb-marker-filter mmixdb-test--stop)
    (should (equal gud-last-frame '("/w/example.mms" . 8)))))

(ert-deftest mmixdb-filter-reassembles-a-marker-split-across-chunks ()
  "A marker arriving in two pieces still resolves to one frame."
  (mmixdb-test--with-marker-state
    (let* ((cut 12)
           (output (concat (gud-mmixdb-marker-filter
                            (substring mmixdb-test--stop 0 cut))
                           (gud-mmixdb-marker-filter
                            (substring mmixdb-test--stop cut)))))
      (should (equal gud-last-frame '("/w/example.mms" . 8)))
      (should (string-match-p "(mmixdb) " output)))))

(ert-deftest mmixdb-marker-regexp-stops-the-path-at-the-line-separator ()
  "Group 1 is the path without the trailing `:LINE'; group 2 is the line."
  (should (string-match gud-mmixdb-marker-regexp mmixdb-test--stop))
  (should (equal (match-string 1 mmixdb-test--stop) "/w/example.mms"))
  (should (equal (match-string 2 mmixdb-test--stop) "8")))

(provide 'mmixdb-test)
;;; mmixdb-test.el ends here
