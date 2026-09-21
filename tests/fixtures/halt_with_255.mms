% halt_with_255.mms - TRAP 0,Halt,0 with $255 = 255: neither 0 nor 1, so
% a build that exits 1 on every halt fails this.
        LOC     #100
Main    SET     $255,255
        TRAP    0,Halt,0
