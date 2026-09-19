% ------------------------------------------------------------
% mmmix.mms -- minimal starting point for an MMIX program
% ------------------------------------------------------------

        LOC     #100            % code segment start
Main    SET     $255,0          % exit code 0, not Main's own address
        TRAP    0,Halt,0        % exit

        LOC     Data_Segment
        GREG    @
