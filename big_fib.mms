% ----------------------------------------------------
% Fibonacci (Big Integer) - bounded arbitrary precision.
% BigInt = MAXLIMBS little-endian 64-bit limbs.
%
% Cross-routine arguments travel via global registers
% Arg0/Arg1/Arg2 ($32/$33/$34) so they survive PUSHJ
% irrespective of the saved-window size.  Single-value
% returns use the standard slide convention: caller does
% PUSHJ $K, callee does POP 1,0, and the returned value
% lands at the caller's $K (the "hole").
% ----------------------------------------------------

Zero    IS      $255
Arg0    IS      $32
Arg1    IS      $33
Arg2    IS      $34
MAXLIMBS IS     32

% ----------------------------------------------------
% Data Segment
% ----------------------------------------------------
        LOC     #1000
ResultMsg BYTE  "fib(100) = ",0
Newline BYTE    10,0

        LOC     #2000
BufA    OCTA    0
        LOC     #2100
BufB    OCTA    0

        LOC     #2200
TempBuf OCTA    0
        LOC     #2400
OutputStr BYTE  0
        LOC     #2800
DigitBuf BYTE   0

% ----------------------------------------------------
% Code Segment
% ----------------------------------------------------
        LOC     #100
        JMP     Main

% ====================================================
% Main: compute fib(100) and print as a decimal string.
% ====================================================
Main    GETA    $6,BufA
        GETA    $7,BufB

        SET     Arg0,$6
        SETI    Arg1,MAXLIMBS
        PUSHJ   $31,ZeroBuf

        SET     Arg0,$7
        SETI    Arg1,MAXLIMBS
        PUSHJ   $31,ZeroBuf

        SETI    Arg0,100
        SET     Arg1,$6
        SET     Arg2,$7
        PUSHJ   $31,Fibonacci           % Arg2 → result buffer

        SET     Arg0,Arg2
        GETA    Arg1,OutputStr
        PUSHJ   $31,BigIntToDecStr

        GETA    $255,ResultMsg
        TRAP    0,Fputs,StdOut
        GETA    $255,OutputStr
        TRAP    0,Fputs,StdOut
        GETA    $255,Newline
        TRAP    0,Fputs,StdOut

        SETI    $255,0
        TRAP    0,Halt,0

% ====================================================
% Subroutines
% ====================================================

% ----------------------------------------------------
% ZeroBuf - zero a buffer of limbs.
% Input: Arg0 = pointer, Arg1 = limb count.
% ----------------------------------------------------
ZeroBuf SETI    $0,0
ZeroLoop
        CMP     $1,$0,Arg1
        BNN     $1,ZeroDone
        SLU     $2,$0,3
        STOU    Zero,Arg0,$2
        ADDU    $0,$0,1
        JMP     ZeroLoop
ZeroDone POP    0,0

% ----------------------------------------------------
% Fibonacci - compute fib(n) in BigInt form.
% Input:  Arg0 = n, Arg1 = bufA (zeroed), Arg2 = bufB (zeroed).
% Output: Arg2 = pointer to the buffer holding fib(n).
% Locals: $0 = n, $5 = A pointer, $6 = B pointer, $7 = swap tmp.
% ----------------------------------------------------
Fibonacci
        SET     $0,Arg0                 % n
        SET     $5,Arg1                 % A
        SET     $6,Arg2                 % B
        BZ      $0,FibN0                % n == 0: result is A (zero)

        SETI    $1,1
        STOU    $1,$6,Zero              % B[0] = 1

        CMP     $2,$0,1
        BZ      $2,FibDone              % n == 1: result is B

        SETI    $3,2                    % i = 2
FibLoop CMP     $4,$3,$0
        BP      $4,FibDone              % i > n: done

        % A := A + B  (in place)
        SET     Arg0,$5
        SET     Arg1,$6
        SET     Arg2,$5
        PUSHJ   $31,MPAddWithCarry

        % Swap (A, B) := (B, A): now B holds the freshest fib.
        SET     $7,$5
        SET     $5,$6
        SET     $6,$7

        ADDU    $3,$3,1
        JMP     FibLoop

FibDone SET     Arg2,$6
        POP     0,0

FibN0   SET     Arg2,$5
        POP     0,0

% ----------------------------------------------------
% MPAddWithCarry - dest := src1 + src2 (multi-precision).
% Input: Arg0 = src1 ptr, Arg1 = src2 ptr, Arg2 = dest ptr.
% Dest may alias src1.
% ----------------------------------------------------
MPAddWithCarry
        SETI    $0,0                    % limb index
        SETI    $1,0                    % running carry
        SETI    $2,MAXLIMBS
MPALoop CMP     $3,$0,$2
        BNN     $3,MPADone

        SLU     $4,$0,3                 % offset = i*8
        LDOU    $5,Arg0,$4
        LDOU    $6,Arg1,$4

        ADDU    $7,$5,$6                % sum without carry-in
        CMPU    $8,$7,$5
        ZSN     $8,$8,1                 % carry-out from sum

        ADDU    $9,$7,$1                % add running carry
        CMPU    $10,$9,$7
        ZSN     $10,$10,1               % carry-out from carry-in add

        STOU    $9,Arg2,$4
        OR      $1,$8,$10               % new running carry

        ADDU    $0,$0,1
        JMP     MPALoop
MPADone POP     0,0

% ----------------------------------------------------
% BigIntToDecStr - convert a BigInt to a decimal string.
% Input: Arg0 = pointer to BigInt
%        Arg1 = pointer to NUL-terminated output buffer.
% ----------------------------------------------------
BigIntToDecStr
        SET     $10,Arg0
        SET     $11,Arg1

        % Copy input to TempBuf so we can destructively divide.
        SETI    $12,0
        GETA    $13,TempBuf
        SETI    $14,MAXLIMBS
CopyLoop
        CMP     $15,$12,$14
        BNN     $15,CopyDone
        SLU     $16,$12,3
        LDOU    $17,$10,$16
        STOU    $17,$13,$16
        ADDU    $12,$12,1
        JMP     CopyLoop
CopyDone

        SETI    $12,0                   % digit count (low-to-high)
ExtractDigits
        % Test whether TempBuf is zero.
        GETA    $13,TempBuf
        SETI    $14,0
        SETI    $15,MAXLIMBS
CheckZero
        CMP     $16,$14,$15
        BNN     $16,IsZero
        SLU     $17,$14,3
        LDOU    $18,$13,$17
        BNZ     $18,NotZero
        ADDU    $14,$14,1
        JMP     CheckZero

IsZero  BZ      $12,WasZero
        JMP     ReverseDigits

WasZero SETI    $16,48
        STBU    $16,$11,Zero
        STBUI   Zero,$11,1
        JMP     BIDSReturn

NotZero SET     Arg0,$13                % stage TempBuf ptr for DivBy10
        PUSHJ   $9,DivBy10              % remainder lands at $9
        ADDU    $9,$9,48                % '0' + remainder
        GETA    $14,DigitBuf
        STBU    $9,$14,$12
        ADDU    $12,$12,1
        JMP     ExtractDigits

ReverseDigits
        SETI    $13,0
RevLoop CMP     $14,$13,$12
        BNN     $14,RevDone
        SUBU    $15,$12,1
        SUBU    $15,$15,$13
        GETA    $16,DigitBuf
        LDBU    $17,$16,$15
        STBU    $17,$11,$13
        ADDU    $13,$13,1
        JMP     RevLoop
RevDone STBU    Zero,$11,$13

BIDSReturn POP  0,0

% ----------------------------------------------------
% DivBy10 - divide BigInt by 10 in place.
% Input:  Arg0 = pointer to BigInt buffer.
% Output: returns remainder (POP 1,0).
% ----------------------------------------------------
DivBy10 SET     $13,Arg0
        SETI    $9,0                    % accumulator (high carry)
        SETI    $28,MAXLIMBS
        SETI    $14,0                   % limb counter (high → low)
DivLoop CMP     $15,$14,$28
        BNN     $15,DivDone
        SUBU    $18,$28,1
        SUBU    $15,$18,$14             % limb index = MAXLIMBS-1-i
        SLU     $15,$15,3
        LDOU    $16,$13,$15

        SETI    $17,0                   % new limb value
        SETI    $18,64                  % bits remaining

BitLoop BZ      $18,LimbDone
        SLU     $9,$9,1
        SETI    $22,63
        SRU     $23,$16,$22
        OR      $9,$9,$23
        SLU     $16,$16,1
        SLU     $17,$17,1

        CMP     $25,$9,10
        BN      $25,SkipSub
        SUBU    $9,$9,10
        OR      $17,$17,1
SkipSub SUBU    $18,$18,1
        JMP     BitLoop

LimbDone STOU   $17,$13,$15
        ADDU    $14,$14,1
        JMP     DivLoop

DivDone SET     $0,$9
        POP     1,0
