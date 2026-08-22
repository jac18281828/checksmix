% ------------------------------------------------------------
% mmmix.mms -- minimal starting point for an MMIX program
% ------------------------------------------------------------

        LOC     #100            % code segment start
Main    TRAP    0,Halt,0        % exit

        LOC     Data_Segment
        GREG    @
