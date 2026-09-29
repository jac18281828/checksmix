% flood_stdout.mms - write "first" to StdOut with Fputs, then 2 MiB of the
% data segment's zero bytes with Fwrite, more than a pipe buffer holds. A
% failed write leaves $255 negative; either way the program halts with 0.

        LOC     Data_Segment
        GREG    @
First   BYTE    "first",10,0
Params  OCTA    0,0

        LOC     #100
Main    LDA     $255,First
        TRAP    0,Fputs,StdOut
        LDA     $1,First
        LDA     $2,Params
        STO     $1,$2,0
        SETML   $3,#20
        STO     $3,$2,8
        SET     $255,$2
        TRAP    0,Fwrite,StdOut
        SET     $255,0
        TRAP    0,Halt,0
