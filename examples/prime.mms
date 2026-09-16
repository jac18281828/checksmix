% Is N prime?  Divide by odd numbers up to sqrt(N).

N       IS      97

        LOC     Data_Segment
        GREG    @
Digits  BYTE    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
End     BYTE    0
Yes     BYTE    " is prime",#a,0
No      BYTE    " is composite",#a,0

        LOC     #100

Main    SET     $1,N
        PUSHJ   $0,IsPrime       % $0 = 1 if prime, 0 if not
        SET     $2,N
        SET     $3,$0
        PUSHJ   $1,Print
        XOR     $255,$0,1        % exit 0 if prime, 1 if composite
        TRAP    0,Halt,0

% IsPrime: n in $0.  Returns 1 if n is prime, else 0.
IsPrime CMPU    $1,$0,2
        BN      $1,Composite     % 0 and 1 are not prime
        BZ      $1,Prime         % 2 is
        AND     $1,$0,1
        BZ      $1,Composite     % no other even number is
        SET     $1,3             % d
Loop    DIVU    $2,$0,$1         % n / d
        GET     $3,rR            % n mod d
        CMPU    $4,$1,$2
        BP      $4,Prime         % d > n/d, so d*d > n
        BZ      $3,Composite
        ADDU    $1,$1,2
        JMP     Loop
Prime   SET     $0,1
        POP     1,0
Composite SET   $0,0
        POP     1,0

% Print: n in $0, verdict in $1.  Digits are written right to left.
Print   LDA     $2,End
        SET     $3,10
Digit   DIVU    $0,$0,$3
        GET     $4,rR
        ADDU    $4,$4,'0'
        SUBU    $2,$2,1
        STBU    $4,$2,0
        PBNZ    $0,Digit
        SET     $255,$2
        TRAP    0,Fputs,StdOut
        LDA     $255,Yes
        BNZ     $1,Say
        LDA     $255,No
Say     TRAP    0,Fputs,StdOut
        POP     0,0
