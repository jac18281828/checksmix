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
        JMP     Print

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

% Print N, then the verdict.  Digits are written right to left.
Print   SET     $2,$1
        LDA     $3,End
        SET     $4,10
Digit   DIVU    $2,$2,$4
        GET     $5,rR
        ADDU    $5,$5,'0'
        SUBU    $3,$3,1
        STBU    $5,$3,0
        PBNZ    $2,Digit
        SET     $255,$3
        TRAP    0,Fputs,StdOut
        LDA     $255,Yes
        BNZ     $0,Say
        LDA     $255,No
Say     TRAP    0,Fputs,StdOut
        SET     $255,0
        TRAP    0,Halt,0
