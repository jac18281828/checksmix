% ----------------------------------------------------
% Function call demo: pass two args via the register
% window slide; receive a sum at the caller's hole.
%
% PUSHJ $X with args staged at $X+1, $X+2 makes them
% visible to the callee as $0, $1.  POP 1 places the
% callee's $0 at the caller's $X (the "hole").
% ----------------------------------------------------
        LOC     #100

Main    SETI    $1,40                   % arg0 → callee's $0
        SETI    $2,2                    % arg1 → callee's $1
        PUSHJ   $0,AddFunc              % result lands at $0
        SET     $255,$0                 % exit code = 42
        TRAP    0,Halt,0

% ----------------------------------------------------
% AddFunc: $0 + $1 -> $0; POP 1 returns to caller's hole.
% ----------------------------------------------------
AddFunc ADDU    $0,$0,$1
        POP     1,0
