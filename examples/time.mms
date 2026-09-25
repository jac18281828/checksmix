% time.mms -- read the host clock and print the Unix time.

        LOC     Data_Segment
        GREG    @
Digits  BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
End     BYTE    0
Suffix  BYTE    " seconds since the Unix epoch",10,0

        LOC     #100

Main    TRAP    0,Time,0                % $255 = seconds since the Unix epoch
        SET     $5,$255                 % stage as PrintNum's $0
        PUSHJ   $4,PrintNum
        LDA     $255,Suffix
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
