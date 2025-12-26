Zero    IS      $255
        LOC     #100

Main    SETI $1,100          
        SETI $2,101
        SETI $3,102
        PUSHJ   $4,AddFunc      % Save $0, $1, $2, $3 (registers 0 through 3)
        % After PUSHJ $4, saved_x = 4
        % After POP 3, return values should be in $4, $5, $6
        
        % Test that $1, $2, $3 were restored
        SETI $254,100
        CMP     $0,$1,$254
        BNZ     $0,Fail         % Fail if $1 != 100 (should be restored)
        SETI $254,101
        CMP     $0,$2,$254
        BNZ     $0,Fail         % Fail if $2 != 101 (should be restored)
        SETI $254,102
        CMP     $0,$3,$254
        BNZ     $0,Fail         % Fail if $3 != 102 (should be restored)
        
        % Test that return values are in $4, $5, $6
        % According to MMIX spec: $4 gets main return ($(X-1) = $2)
        % $5 gets $0, $6 gets $1
        SETI $254,302        % Expected main return value from $2
        CMP     $0,$4,$254
        BNZ     $0,Fail         % Fail if $4 != 302
        SETI $254,300        % Expected return value from $0
        CMP     $0,$5,$254
        BNZ     $0,Fail         % Fail if $5 != 300
        SETI $254,301        % Expected return value from $1
        CMP     $0,$6,$254
        BNZ     $0,Fail         % Fail if $6 != 301
        
        SETI $0,0            % indicate success
        JMP     Done

AddFunc % Function that computes 3 return values
        SETI $0,300          % Return value 0
        SETI $1,301          % Return value 1
        SETI $2,302          % Main return value (will be in $(X-1))
        POP     3,0             % Return 3 values: $0, $1, $2
        
Fail    SETI $0,1            % indicate failure
        JMP     Done

Done    TRAP    0,Halt,0
