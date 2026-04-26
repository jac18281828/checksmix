; multi_main.mms - calls into :Lib defined in multi_lib.mms
        LOC     #100
Main    PUSHJ   $0,:Lib
        TRAP    0,Halt,0
