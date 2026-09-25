% exit_code.mms -- return a value to the shell, halts with 42.

        LOC     #100
Main    SETI    $255, 42        % Set exit code to 42
        TRAP    0, Halt, 0      % HALT with exit code 42
