% echo.mms - read one line from StdIn with Fgets and echo it to StdOut. A
% failed read ($255 negative) prints a marker instead, so a test can tell
% the two cases apart without inspecting the exit code.

        LOC     Data_Segment
        GREG    @
Buffer  BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
        BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
        BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
        BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
Params  OCTA    0,0
Marker  BYTE    "no input",10,0

        LOC     #100
Main    LDA     $1,Buffer
        LDA     $2,Params
        STO     $1,$2,0
        SET     $3,64
        STO     $3,$2,8
        SET     $255,$2
        TRAP    0,Fgets,StdIn
        BN      $255,Failed
        SET     $255,$1
        TRAP    0,Fputs,StdOut
        SET     $255,0
        TRAP    0,Halt,0
Failed  LDA     $255,Marker
        TRAP    0,Fputs,StdOut
        SET     $255,0
        TRAP    0,Halt,0
