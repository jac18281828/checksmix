% overflowing_byte.mms - one BYTE item wider than a byte; warns and keeps
% its low byte, but still halts cleanly with exit code 0.
        LOC     #100
Main    TRAP    0,Halt,0
        BYTE    300
