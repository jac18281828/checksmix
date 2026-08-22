% =====================================================================
% pi.mms -- Compute pi as 4*atan(1) using the Atan library and print it.
% =====================================================================
%
% Build/run: checksmix run pi.mms
%
% atan(1) = pi/4 exactly along this routine's code path (x=1.0 skips the
% reciprocal reduction, shift-reduces to u=0, and Horner(0) is exact), so
% this is one of the few inputs where the truncated-series approximation
% error is zero -- see atan_test.mms's notes on accuracy elsewhere.
% =====================================================================

        INCLUDE atan.mms

        LOC     #5000                   % near code; keeps data within GETA's reach
Buf      BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
         BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0

MsgHdr   BYTE    "pi = 4*atan(1) = ",0
MsgNL    BYTE    #a,0

        LOC     #1000

% PrintHex: print a NUL-terminated 16-digit lowercase-hex of the value
% in $0 to stdout. Uses Buf.
PrintHex IS     @
        GETA    $1,Buf
        SETL    $2,16
        SETL    $3,0
:PHLoop CMP     $4,$3,$2
        BNN     $4,:PHEmit
        SUBU    $6,$2,$3
        SUBU    $6,$6,1
        SLU     $6,$6,2
        SRU     $7,$0,$6
        AND     $7,$7,#F
        CMP     $4,$7,10
        BN      $4,:PHDigit
        ADDU    $7,$7,55
        JMP     :PHStore
:PHDigit ADDU   $7,$7,'0'
:PHStore STBU  $7,$1,$3
        ADDU    $3,$3,1
        JMP     :PHLoop
:PHEmit SETL    $4,0
        STBUI   $4,$1,16
        SET     $255,$1
        TRAP    0,Fputs,StdOut
        POP     0,0

Main    LDA     $255,MsgHdr
        TRAP    0,Fputs,StdOut

        % pi/4 = atan(1) via the standard PUSHJ convention.
        SETH    $21,#3FF0
        PUSHJ   $20,:Atan                % $21 = 1.0 bits, result -> $20
        FADD    $0,$20,$20               % 2*atan(1)
        FADD    $0,$0,$0                 % 4*atan(1) = pi

        % PUSHJ $30 slides the callee's $0 in from the CALLER's $31 (X+1),
        % not from the caller's own $0 -- so the argument must be staged
        % one register above the hole.
        SET     $31,$0
        PUSHJ   $30,PrintHex

        LDA     $31,MsgNL
        PUSHJ   $30,PrintStr
        SETL    $255,0                   % Fputs leaves its byte count in $255;
        TRAP    0,Halt,0                 % clear it so Halt reports exit code 0.

% PrintStr: print NUL-terminated string whose address is in $0.
PrintStr IS     @
        SET     $255,$0
        TRAP    0,Fputs,StdOut
        POP     0,0
