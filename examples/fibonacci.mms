% ----------------------------------------------------
% Fibonacci Sequence 
% ----------------------------------------------------
        LOC     #100
% Entry point
Main    SETI $1,20              % compute fib(20)
        PUSHJ   $0,Fibonacci    % call Fibonacci
        JMP     Done

        % ----------------------------------------------------
        % Program segment
        % ----------------------------------------------------


        % ----------------------------------------------------
        % Registers used:
        %   $0 = result
        %   $1 = n (input)
        %   $2 = fib(n-2)
        %   $3 = fib(n-1)
        %   $4 = counter
        %   $5 = temp
        % ----------------------------------------------------        

% calculate fib($1) and return result in $0
Fibonacci
        CMPI    $5,$1,2
        BN      $5,TwoOrLess
        SETI $2,0
        SETI $3,1
        SETI $4,2
AddLoop
        CMP     $5,$4,$1
        BP      $5,FibEnd
        ADDU    $5,$2,$3
        SET     $2,$3
        SET     $3,$5
        ADDUI   $4,$4,1
        JMP     AddLoop
FibEnd
        SET     $0,$3
        POP     0,0             % return to caller (rJ)
TwoOrLess
        SET     $0,$1
        POP     0,0             % return to caller (rJ)

Done
        TRAP    0,Halt,0
