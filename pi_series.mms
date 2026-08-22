% ---------------------------------------------------------------
% pi_series.mms -- pi = 4*atan(1), the crude way.
%
%   atan(1) = 1 - 1/3 + 1/5 - 1/7 + ...   (Gregory's series at x=1)
%
% All arithmetic is fixed point, scaled by 10^9.
% ---------------------------------------------------------------

Scale   IS      1000000000              % fixed-point scale, 10^9
Terms   IS      100000                  % series terms to sum

Sum     IS      $1                      % running total, scaled
Den     IS      $2                      % denominator: 1, 3, 5, 7, ...
Term    IS      $3                      % Scale/Den
Neg     IS      $4                      % 0 = add this term, 1 = subtract
Cnt     IS      $5                      % terms remaining
One     IS      $6                      % Scale, in a register

        LOC     #4000                   % near code; keeps Buf within GETA's reach
Buf     BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
Hdr     BYTE    "pi ~= ",0

        LOC     #100

Main    SETI    Sum,0
        SETI    Den,1
        SETI    Neg,0
        SETI    One,Scale
        SETI    Cnt,Terms

Loop    DIV     Term,One,Den            % Term = Scale/Den
        BNZ     Neg,Minus
        ADDU    Sum,Sum,Term
        JMP     Step
Minus   SUBU    Sum,Sum,Term
Step    XOR     Neg,Neg,1               % alternate the sign
        ADDU    Den,Den,2               % next odd denominator
        SUBU    Cnt,Cnt,1
        BNZ     Cnt,Loop

        MULU    Sum,Sum,4               % pi = 4 * atan(1)

        LDA     $255,Hdr
        TRAP    0,Fputs,StdOut
        SET     $31,Sum                 % PUSHJ $30 slides $31 in as the callee's $0
        PUSHJ   $30,PrintFix
        SETL    $255,0
        TRAP    0,Halt,0

% PrintFix: $0 holds a value scaled by 10^9, in [10^9, 10^10).
% Prints it as d.ddddddddd and a newline.
PrintFix IS     @
        GETA    $1,Buf
        SETI    $2,10                   % byte index; digits fill backwards
:PFLoop DIV     $3,$0,10
        MULU    $4,$3,10
        SUBU    $4,$0,$4                % $4 = $0 mod 10
        ADDU    $4,$4,'0'
        STBU    $4,$1,$2
        SET     $0,$3
        SUBU    $2,$2,1
        CMP     $5,$2,2
        BNN     $5,:PFLoop
        SETI    $4,'.'
        STBUI   $4,$1,1
        ADDU    $4,$0,'0'
        STBUI   $4,$1,0                 % the leading digit is what's left
        SETI    $4,#a
        STBUI   $4,$1,11
        SETI    $4,0
        STBUI   $4,$1,12
        SET     $255,$1
        TRAP    0,Fputs,StdOut
        POP     0,0
