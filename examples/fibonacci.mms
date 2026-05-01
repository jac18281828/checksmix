% ----------------------------------------------------
% Fibonacci - iterative; demonstrates the standard MMIX
% calling convention with the register-stack window slide.
%
% Caller stages args at $X+1, $X+2, ...; the callee sees
% them at $0, $1, ...  POP n places n return values at the
% caller's $X..$X+n-1 (the "hole").
% ----------------------------------------------------

        LOC     #100

% Entry point: compute fib(20) and exit with the result as the exit code.
Main    SETI    $1,20                   % stage arg at $X+1 = $1
        PUSHJ   $0,Fibonacci            % result lands at $0 after POP 1
        SET     $255,$0                 % return code = fib(20)
        TRAP    0,Halt,0

% ----------------------------------------------------
% Fibonacci - iterative two-register accumulator
% Input:  $0 = n (slid in from caller's $X+1)
% Output: $0 = fib(n) (placed at caller's $X by POP 1)
% Locals: $0 = n / result, $1 = a, $2 = b, $3 = i, $4 = tmp
% ----------------------------------------------------
Fibonacci
        CMP     $4,$0,2
        BN      $4,FibSmall             % n < 2: return n unchanged

        SETI    $1,0                    % a = fib(0)
        SETI    $2,1                    % b = fib(1)
        SETI    $3,2                    % i = 2
FibLoop
        ADDU    $4,$1,$2                % tmp = a + b
        SET     $1,$2                   % a = b
        SET     $2,$4                   % b = tmp
        ADDU    $3,$3,1
        CMP     $4,$3,$0
        BNP     $4,FibLoop              % while i <= n

        SET     $0,$2                   % result = b
FibSmall
        POP     1,0                     % return $0 to caller's hole
