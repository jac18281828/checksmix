% hello_halt.mms -- Hello, Halt: the smallest program that runs.

    LOC #100
Main
    SET     $255,0
    TRAP    0,Halt,0
