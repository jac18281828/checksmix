% leapyear.mms -- the first leap year after Year, by the Gregorian rule.

Year    IS      2026

        LOC     Data_Segment
        GREG    @
Digits  BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
End     BYTE    0
After   BYTE    "The first leap year after ",0
Is      BYTE    " is ",0
Newline BYTE    10,0

        LOC     #100
Main    SET     $1,Year
        ADDU    $2,$1,1                 % the candidate year
        SET     $6,400
        SET     $7,100
1H      DIVU    $3,$2,$6
        GET     $3,rR
        BZ      $3,Found                % every 400th year is leap
        DIVU    $3,$2,$7
        GET     $3,rR
        BZ      $3,2F                   % any other century is not
        AND     $3,$2,3
        BZ      $3,Found                % otherwise every 4th year is
2H      ADDU    $2,$2,1
        JMP     1B

Found   LDA     $255,After
        TRAP    0,Fputs,StdOut
        SET     $5,$1
        PUSHJ   $4,PrintNum
        LDA     $255,Is
        TRAP    0,Fputs,StdOut
        SET     $5,$2
        PUSHJ   $4,PrintNum
        LDA     $255,Newline
        TRAP    0,Fputs,StdOut
        SET     $255,0
        TRAP    0,Halt,0

% PrintNum: write $0 in decimal, filling Digits from the right.
PrintNum LDA    $1,End
        SET     $2,10
1H      DIVU    $0,$0,$2
        GET     $3,rR
        ADDU    $3,$3,'0'
        SUBU    $1,$1,1
        STBU    $3,$1,0
        PBNZ    $0,1B
        SET     $255,$1
        TRAP    0,Fputs,StdOut
        POP     0,0
