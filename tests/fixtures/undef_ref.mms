; undef_ref.mms - references an undefined label; should fail to assemble
        LOC     #100
Main    PUSHJ   $0,NoSuchLabel
        TRAP    0,Halt,0
