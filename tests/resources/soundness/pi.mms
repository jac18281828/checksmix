% Pi to 1000 decimals by Machin's formula, pi = 16*arctan(1/5) -
% 4*arctan(1/239), each arctan its alternating series 1/x - 1/(3x^3) +
% 1/(5x^5) - ... The digits are known from two independent computations
% of Machin's formula at higher precision, agreeing with each other.
%
% A number is a Words-word unsigned fixed-point value: word[0] is the
% integer part, word[1..Words-1] the fraction, most significant word
% first. Dividing by a one-word divisor is long division from word[0],
% the running remainder carried rD -> DIVU -> rR from one word into the
% next (PUT/GET). A decimal digit comes from multiplying the fractional
% words by 10 with MULU and carrying rH the same way, from the least
% significant word up.
%
% Every arctan term is folded into PiAcc directly, term by term, in
% alternating sign; PiAcc only ever holds pi's own running partial sum,
% which stays positive throughout, so every subtraction is well-formed.
%
% Register rules follow examples/big_fib.mms: arguments travel in the
% global registers Arg0-Arg2; any call may overwrite them, so a value
% needed after a call sits in a local below that call's hole.

Words   IS      60

Zero    GREG    0
Arg0    GREG    0
Arg1    GREG    0
Arg2    GREG    0

        LOC     #1000
PiAcc   OCTA    0                       % Words octas: pi's running sum
        LOC     #1200
Power   OCTA    0                       % Words octas: the current 1/x^(2k+1)
        LOC     #1400
Term    OCTA    0                       % Words octas: this round's term

        LOC     #1600
DotStr  BYTE    ".",0
        LOC     #1610
NLStr   BYTE    10,0
        LOC     #1620
DigitChar BYTE  0,0
        LOC     #1630
DecBuf  BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
        LOC     #1650
DecEnd  BYTE    0

        LOC     #100
        JMP     Main

% ====================================================
% Main: pi = 16*arctan(1/5) - 4*arctan(1/239), printed as "3." then
% 1000 fractional decimal digits and a newline.
% ====================================================
Main    GETA    Arg0,PiAcc
        PUSHJ   $10,BigZero

        SET     Arg0,5
        SET     Arg1,16
        SET     Arg2,1                  % k=0 adds
        PUSHJ   $10,ArctanSeries

        SET     Arg0,239
        SET     Arg1,4
        SET     Arg2,0                  % k=0 subtracts
        PUSHJ   $10,ArctanSeries

        GETA    $1,PiAcc
        LDOU    $2,$1,0
        SET     Arg0,$2
        PUSHJ   $10,PrintSmallDecimal
        GETA    $255,DotStr
        TRAP    0,Fputs,StdOut

        SET     $30,0
        SET     $32,1000
DigitLp CMPU    $31,$30,$32
        BNN     $31,DigitDone

        GETA    $1,PiAcc
        SET     $2,0                    % carry
        SET     $3,Words-1              % word index, MSW of the fraction last
DigitWordLp
        SLU     $4,$3,3
        LDOU    $5,$1,$4
        MULU    $6,$5,10
        GET     $7,rH
        ADDU    $8,$6,$2
        CMPU    $9,$8,$6
        ZSN     $9,$9,1
        ADDU    $2,$7,$9
        STOU    $8,$1,$4
        CMPU    $11,$3,1
        BZ      $11,DigitWordDone
        SUBU    $3,$3,1
        JMP     DigitWordLp
DigitWordDone

        ADDU    $12,$2,'0'
        GETA    $13,DigitChar
        STBU    $12,$13,0
        SET     $255,$13
        TRAP    0,Fputs,StdOut

        ADDU    $30,$30,1
        JMP     DigitLp
DigitDone

        GETA    $255,NLStr
        TRAP    0,Fputs,StdOut

        SET     $255,0
        TRAP    0,Halt,0

% ====================================================
% ArctanSeries: Arg0 = x, Arg1 = C, Arg2 = 1 if the k=0 term adds to
% PiAcc, 0 if it subtracts. Adds C*arctan(1/x) into PiAcc, term by
% term, stopping once Power underflows to zero at Words-word width.
% Locals: $0 = saved rJ, $1 = x, $4 = sign flag, $5 = k, $7 = x^2 --
% all below hole $10, so every nested call leaves them untouched.
% ====================================================
ArctanSeries GET $0,rJ
        SET     $1,Arg0
        SET     $2,Arg1
        SET     $6,Arg2

        MULU    $7,$1,$1                % x^2

        GETA    Arg0,Power
        PUSHJ   $10,BigZero
        GETA    $8,Power
        STOU    $2,$8,0                 % Power[0] = C

        GETA    Arg0,Power
        GETA    Arg1,Power
        SET     Arg2,$1
        PUSHJ   $10,BigDivSmall         % Power = C / x

        SET     $4,$6
        SET     $5,0

SeriesLp GETA   Arg0,Power
        PUSHJ   $10,BigIsZero
        BZ      $10,SeriesDone

        GETA    Arg0,Power
        GETA    Arg1,Term
        SLU     $9,$5,1
        ADDU    $9,$9,1
        SET     Arg2,$9
        PUSHJ   $10,BigDivSmall         % Term = Power / (2k+1)

        GETA    Arg0,PiAcc
        GETA    Arg1,Term
        BZ      $4,SubTerm
        PUSHJ   $10,BigAdd
        JMP     SignDone
SubTerm PUSHJ   $10,BigSub
SignDone

        GETA    Arg0,Power
        GETA    Arg1,Power
        SET     Arg2,$7
        PUSHJ   $10,BigDivSmall         % Power /= x^2

        XOR     $4,$4,1
        ADDU    $5,$5,1
        JMP     SeriesLp
SeriesDone
        PUT     rJ,$0
        POP     0,0

% ====================================================
% BigZero: Arg0 = pointer. Zeros a Words-word bignum.
% ====================================================
BigZero SET     $1,Arg0
        SET     $2,0
BZLoop  CMPU    $3,$2,Words
        BNN     $3,BZDone
        SLU     $4,$2,3
        STOU    Zero,$1,$4
        ADDU    $2,$2,1
        JMP     BZLoop
BZDone  POP     0,0

% ====================================================
% BigDivSmall: Arg0 = src, Arg1 = dst, Arg2 = divisor (a word).
% dst = floor(src / divisor), by long division from word[0]; the
% remainder carries word to word through rD/rR.
% ====================================================
BigDivSmall SET $1,Arg0
        SET     $2,Arg1
        SET     $3,Arg2
        PUT     rD,0
        SET     $4,0
BDSLoop CMPU    $5,$4,Words
        BNN     $5,BDSDone
        SLU     $6,$4,3
        LDOU    $7,$1,$6
        DIVU    $8,$7,$3
        STOU    $8,$2,$6
        GET     $9,rR
        PUT     rD,$9
        ADDU    $4,$4,1
        JMP     BDSLoop
BDSDone POP     0,0

% ====================================================
% BigIsZero: Arg0 = pointer. Returns the OR of every word -- zero iff
% the bignum is zero.
% ====================================================
BigIsZero SET   $1,Arg0
        SET     $2,0
        SET     $3,0
BIZLoop CMPU    $4,$2,Words
        BNN     $4,BIZDone
        SLU     $5,$2,3
        LDOU    $6,$1,$5
        OR      $3,$3,$6
        ADDU    $2,$2,1
        JMP     BIZLoop
BIZDone SET     $0,$3
        POP     1,0

% ====================================================
% BigAdd: Arg0 = dst, Arg1 = src. dst += src, word by word from the
% least significant word up, carry propagated by comparing the sum
% against an operand (unsigned wraparound means the sum can't grow).
% ====================================================
BigAdd  SET     $1,Arg0
        SET     $2,Arg1
        SET     $3,0                    % carry
        SET     $4,Words-1
BAddLp  SLU     $5,$4,3
        LDOU    $6,$1,$5
        LDOU    $7,$2,$5
        ADDU    $8,$6,$7
        CMPU    $9,$8,$6
        ZSN     $9,$9,1
        ADDU    $10,$8,$3
        CMPU    $11,$10,$8
        ZSN     $11,$11,1
        OR      $3,$9,$11
        STOU    $10,$1,$5
        BZ      $4,BAddDone
        SUBU    $4,$4,1
        JMP     BAddLp
BAddDone POP    0,0

% ====================================================
% BigSub: Arg0 = dst, Arg1 = src. dst -= src, word by word from the
% least significant word up, with the matching borrow chain. Every
% call site keeps dst >= src, so no run ever needs the result to wrap.
% ====================================================
BigSub  SET     $1,Arg0
        SET     $2,Arg1
        SET     $3,0                    % borrow
        SET     $4,Words-1
BSubLp  SLU     $5,$4,3
        LDOU    $6,$1,$5
        LDOU    $7,$2,$5
        SUBU    $8,$6,$7
        CMPU    $9,$6,$7
        ZSN     $9,$9,1
        SUBU    $10,$8,$3
        CMPU    $11,$8,$3
        ZSN     $11,$11,1
        OR      $3,$9,$11
        STOU    $10,$1,$5
        BZ      $4,BSubDone
        SUBU    $4,$4,1
        JMP     BSubLp
BSubDone POP    0,0

% ====================================================
% PrintSmallDecimal: Arg0 = value, printed as unsigned decimal.
% ====================================================
PrintSmallDecimal GETA $1,DecEnd
        SET     $0,Arg0
        SET     $2,10
        PUT     rD,0
PSDLoop DIVU    $3,$0,$2
        GET     $4,rR
        ADDU    $4,$4,'0'
        SUBU    $1,$1,1
        STBU    $4,$1,0
        SET     $0,$3
        PBNZ    $0,PSDLoop
        SET     $255,$1
        TRAP    0,Fputs,StdOut
        POP     0,0
