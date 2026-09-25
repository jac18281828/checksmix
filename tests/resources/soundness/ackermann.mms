% Ackermann-Peter function, A(0,n)=n+1, A(m,0)=A(m-1,1),
% A(m,n)=A(m-1,A(m,n-1)), printed for m = 0..3, n = 0..5. Each value is
% known from the closed forms A(0,n)=n+1, A(1,n)=n+2, A(2,n)=2n+3,
% A(3,n)=2^(n+3)-3.
%
% Every call is PUSHJ/POP, never a jump: the recursion nests A(3,5) 255
% register-stack frames deep, the depth PUSHJ/POP has to restore correctly
% for the printed values to match the closed forms.
%
% Register rules:
% - Arguments travel in the global registers Arg0, Arg1; any call may
%   overwrite them, so a value needed after a call is kept below that
%   call's hole instead.
% - Ackermann keeps its own m in $0, its caller's rJ in $2, and calls
%   itself through hole $3: $0..$2 survive every nested call untouched.

Arg0    GREG    0
Arg1    GREG    0

        LOC     #1000
OpenStr BYTE    "A(",0
        LOC     #1010
CommaStr BYTE   ",",0
        LOC     #1020
EqStr   BYTE    ") = ",0
        LOC     #1030
NLStr   BYTE    10,0
        LOC     #1040
DecBuf  BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
        LOC     #1060
DecEnd  BYTE    0

        LOC     #100
        JMP     Main

% ====================================================
% Main: print "A(m,n) = value" for m = 0..3, n = 0..5.
% Locals: $10 = m, $11 = n; hole $20 for every call, so both survive.
% ====================================================
Main    SET     $10,0
OuterLoop CMPU  $12,$10,4
        BNN     $12,Done

        SET     $11,0
InnerLoop CMPU  $12,$11,6
        BNN     $12,NextM

        GETA    $255,OpenStr
        TRAP    0,Fputs,StdOut
        SET     Arg0,$10
        PUSHJ   $20,PrintDecimal

        GETA    $255,CommaStr
        TRAP    0,Fputs,StdOut
        SET     Arg0,$11
        PUSHJ   $20,PrintDecimal

        GETA    $255,EqStr
        TRAP    0,Fputs,StdOut

        SET     Arg0,$10
        SET     Arg1,$11
        PUSHJ   $20,Ackermann
        SET     Arg0,$20
        PUSHJ   $20,PrintDecimal

        GETA    $255,NLStr
        TRAP    0,Fputs,StdOut

        ADDU    $11,$11,1
        JMP     InnerLoop

NextM   ADDU    $10,$10,1
        JMP     OuterLoop

Done    SET     $255,0
        TRAP    0,Halt,0

% ====================================================
% Ackermann: Arg0 = m, Arg1 = n. Returns A(m,n) in the caller's hole.
% Locals: $0 = m, $1 = n, $2 = saved rJ; the recursive hole is $3, so
% $0..$2 come back from a nested call exactly as they went in.
% ====================================================
Ackermann GET   $2,rJ
        SET     $0,Arg0
        SET     $1,Arg1
        BZ      $0,AckBaseM
        BZ      $1,AckBaseN

        SET     Arg0,$0
        SUBU    Arg1,$1,1
        PUSHJ   $3,Ackermann            % $3 = A(m, n-1)

        SUBU    Arg0,$0,1
        SET     Arg1,$3
        PUSHJ   $3,Ackermann            % $3 = A(m-1, A(m,n-1))

        SET     $0,$3
        PUT     rJ,$2
        POP     1,0

AckBaseN SUBU   Arg0,$0,1
        SET     Arg1,1
        PUSHJ   $3,Ackermann            % $3 = A(m-1, 1)

        SET     $0,$3
        PUT     rJ,$2
        POP     1,0

AckBaseM SET    $0,$1
        ADDU    $0,$0,1
        POP     1,0

% ====================================================
% PrintDecimal: Arg0 = value, printed as unsigned decimal, no newline.
% ====================================================
PrintDecimal GETA $1,DecEnd
        SET     $0,Arg0
        SET     $2,10
        PUT     rD,0
PDLoop  DIVU    $3,$0,$2
        GET     $4,rR
        ADDU    $4,$4,'0'
        SUBU    $1,$1,1
        STBU    $4,$1,0
        SET     $0,$3
        PBNZ    $0,PDLoop

        SET     $255,$1
        TRAP    0,Fputs,StdOut
        POP     0,0
