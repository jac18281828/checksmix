% echo_loop.mms - read lines from StdIn with Fgets in a loop, echoing each
% to StdOut, until a read fails; then print a marker and halt. The loop's
% top is the entry point, so a breakpoint there stops execution once per
% line under a debugger.

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
        JMP     Main
Failed  LDA     $255,Marker
        TRAP    0,Fputs,StdOut
        SET     $255,0
        TRAP    0,Halt,0
