% greg_postamble.mms - a minimal program with one GREG, for confirming a
% built .mmo's postamble carries the register it allocates.

        LOC     #100
Base    GREG    1000
Main    TRAP    0,Halt,0
