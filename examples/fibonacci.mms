% fibonacci.mms -- compute fib(20) in a loop and print the result.
%
% Caller stages arguments at $X+1, $X+2, ...; PUSHJ $X saves $0..$X-1
% and the callee sees the arguments as $0, $1, ....
% POP 1,0 puts the callee's $0 in the caller's $X (the hole), restores
% $0..$X-1, and every register above $X reads zero.
% POP 2,0 is not in order: the hole gets the callee's $1, $X+1 gets $0.

        LOC     Data_Segment
        GREG    @
Digits  BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
End     BYTE    0
Msg     BYTE    "fib(20) = ",0
Newline BYTE    10,0

        LOC     #100

% Entry point: compute fib(20) and print the result.
Main    SETI    $1,20                   % stage arg at $X+1 = $1
        PUSHJ   $0,Fibonacci            % result lands at $0 after POP 1
        LDA     $255,Msg
        TRAP    0,Fputs,StdOut
        SET     $5,$0                   % stage fib(20) as PrintNum's $0
        PUSHJ   $4,PrintNum
        LDA     $255,Newline
        TRAP    0,Fputs,StdOut
        SET     $255,0
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

% PrintNum: write $0 in decimal, filling Digits from the right.
PrintNum LDA    $1,End
        SET     $2,10
1H      DIVU    $0,$0,$2
        GET     $3,rR
        ADDU    $3,$3,'0'
        SUBU    $1,$1,1
        STBU    $3,$1,0
        PBNZ    $0,1B
        SET     $255,$1
        TRAP    0,Fputs,StdOut
        POP     0,0
