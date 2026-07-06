; include_main.mms - pulls in include_lib.mms via INCLUDE, then calls into it
        LOC     #100
INCLUDE include_lib.mms
Main    PUSHJ   $0,:LibHalt
