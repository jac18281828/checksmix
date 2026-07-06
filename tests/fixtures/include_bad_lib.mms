; include_bad_lib.mms - deliberately malformed, to prove per-file diagnostics:
; a parse error inside an included file must report THIS file's name, not
; the including file's, proving translation-unit reuse rather than splicing.
        THIS_IS_NOT_A_REAL_MNEMONIC $0,$1,$2
