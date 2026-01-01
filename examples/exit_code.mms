% Test program to verify exit code handling
Main    SETI    $255, 42        % Set exit code to 42
        TRAP    0, Halt, 0      % HALT with exit code 42
