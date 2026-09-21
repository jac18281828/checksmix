% put_rc_halts.mms - PUT rC is a privileged-operation interrupt: a
% diagnostic halt, distinct from TRAP 0,Halt,0.
        LOC     #100
Main    SET     $1,5
        PUT     rC,$1
        TRAP    0,Halt,0
