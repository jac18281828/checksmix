% ----------------------------------------------------
% Primality by trial division, iterative.
%
% N is composite exactly when some divisor D in
% [2, sqrt(N)] divides it.  Rather than square D to
% find that bound, compare D against the quotient the
% division already produced: D > N/D holds precisely
% when D*D > N, and it cannot overflow.
%
% Even N are settled by their low bit, so the scan
% visits only odd divisors.
%
% $1 = N   $2 = D   $3 = N/D   $4 = N mod D   $9 = verdict
%
% N above #FFFF needs SETI in place of SET.
% ----------------------------------------------------
N       IS      97

        LOC     Data_Segment
        GREG    @
Digits  BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
End     BYTE    0
Yes     BYTE    " is prime",#a,0
No      BYTE    " is composite",#a,0

        LOC     #100

Main    SET     $1,N
        PUT     rD,0                    % DIVU divides rD:$Y; keep the high half clear
        SET     $0,2
        CMPU    $0,$1,$0
        BN      $0,Composite            % 0 and 1 are neither
        BZ      $0,Prime                % 2 is the one even prime
        AND     $0,$1,1
        BZ      $0,Composite            % any larger even N has the factor 2
        SET     $2,3

Trial   DIVU    $3,$1,$2                % one division yields both the
        GET     $4,rR                   % quotient bound and the remainder
        CMPU    $0,$2,$3
        BP      $0,Prime                % D > N/D: the scan has passed sqrt(N)
        BZ      $4,Composite            % D divides N
        ADDU    $2,$2,2
        JMP     Trial

Prime   SET     $9,1
        JMP     Report

Composite
        SET     $9,0

% ----------------------------------------------------
% Report: print N in decimal, then the verdict.
% Digits are generated least significant first, so the
% string is built backwards from its terminating byte.
% ----------------------------------------------------
Report  SET     $4,$1
        LDA     $5,End
        SET     $6,10

Digit   DIVU    $7,$4,$6
        GET     $8,rR
        ADDU    $8,$8,'0'
        SUBU    $5,$5,1
        STBU    $8,$5,0
        SET     $4,$7
        PBNZ    $4,Digit

        SET     $255,$5
        TRAP    0,Fputs,StdOut
        BZ      $9,SayNo
        LDA     $255,Yes
        JMP     Say

SayNo   LDA     $255,No

Say     TRAP    0,Fputs,StdOut
        SET     $255,0                  % Fputs leaves its byte count in $255
        TRAP    0,Halt,0
