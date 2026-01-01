        LOC     #100

Main    SETI $1,100          
        SETI $2,101
        SETI $3,102
        PUSHJ   $4,AddFunc      % Save $0, $1, $2, $3 (registers 0 through 3)
        % After PUSHJ $4, saved_count = 4
        % After POP 3, return values are in $0, $1, $2 (NOT moved to $4, $5, $6)
        
        % Test that return values are in $0, $1, $2
        SETI $254,300        % Expected return value from $0
        CMP     $0,$0,$254
        BNZ     $0,Fail         % Fail if $0 != 300
        SETI $254,301        % Expected return value from $1
        CMP     $0,$1,$254
        BNZ     $0,Fail         % Fail if $1 != 301
        SETI $254,302        % Expected main return value from $2
        CMP     $0,$2,$254
        BNZ     $0,Fail         % Fail if $2 != 302
        
        % Test that $3 was restored (only $X and above are restored, so $0-$2 kept, $3+ restored)
        SETI $254,102
        CMP     $0,$3,$254
        BNZ     $0,Fail         % Fail if $3 != 102 (should be restored)
        
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
