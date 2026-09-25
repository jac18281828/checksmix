% SHA-256 (FIPS 180-4) over three messages, each digest printed as 64
% lowercase hex digits on its own line: the empty string, "abc", and a
% 56-byte message whose padding spills into a second 64-byte block.
% Each digest is known from NIST's published example digests for these
% three messages.
%
% H and K are the first 32 bits of the fractional part of the square
% roots of the first 8 primes and the cube roots of the first 64 primes
% (FIPS 180-4 4.2.2).
%
% Word arithmetic is mod 2^32: every register that holds a message word
% is masked to Mask32 right after an addition, so the rotate identities
% below stay exact. Ch(x,y,z) is one MUX with rM = x -- MUX picks Y where
% the mask bit is 1 and Z where it is 0, which is exactly x?y:z bitwise.
%
% Register rules follow examples/big_fib.mms: arguments travel in the
% global registers Arg0, Arg1; any call may overwrite them, so a value
% needed after a call sits in a local below that call's hole.

Zero    GREG    0
Arg0    GREG    0
Arg1    GREG    0
Mask32  GREG    #FFFFFFFF

        LOC     #1000
H0table TETRA   #6a09e667,#bb67ae85,#3c6ef372,#a54ff53a
        TETRA   #510e527f,#9b05688c,#1f83d9ab,#5be0cd19

        LOC     #1100
Ktable  TETRA   #428a2f98,#71374491,#b5c0fbcf,#e9b5dba5
        TETRA   #3956c25b,#59f111f1,#923f82a4,#ab1c5ed5
        TETRA   #d807aa98,#12835b01,#243185be,#550c7dc3
        TETRA   #72be5d74,#80deb1fe,#9bdc06a7,#c19bf174
        TETRA   #e49b69c1,#efbe4786,#0fc19dc6,#240ca1cc
        TETRA   #2de92c6f,#4a7484aa,#5cb0a9dc,#76f988da
        TETRA   #983e5152,#a831c66d,#b00327c8,#bf597fc7
        TETRA   #c6e00bf3,#d5a79147,#06ca6351,#14292967
        TETRA   #27b70a85,#2e1b2138,#4d2c6dfc,#53380d13
        TETRA   #650a7354,#766a0abb,#81c2c92e,#92722c85
        TETRA   #a2bfe8a1,#a81a664b,#c24b8b70,#c76c51a3
        TETRA   #d192e819,#d6990624,#f40e3585,#106aa070
        TETRA   #19a4c116,#1e376c08,#2748774c,#34b0bcb5
        TETRA   #391c0cb3,#4ed8aa4a,#5b9cca4f,#682e6ff3
        TETRA   #748f82ee,#78a5636f,#84c87814,#8cc70208
        TETRA   #90befffa,#a4506ceb,#bef9a3f7,#c67178f2

        LOC     #1300
Msg1    BYTE    0
        LOC     #1310
Msg2    BYTE    "abc"
        LOC     #1320
Msg3    BYTE    "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"

        LOC     #1400
PadBuf  BYTE    0                       % up to 128 bytes, two blocks

        LOC     #1500
WBuf    BYTE    0                       % 64 tetras, the message schedule

        LOC     #1600
Hstate  BYTE    0                       % 8 tetras, the running hash

        LOC     #1700
HexDigits BYTE  "0123456789abcdef"

        LOC     #1710
HexOut  BYTE    0                       % 64 hex chars + newline + NUL

        LOC     #100
        JMP     Main

% ====================================================
% Main: run each message through Sha256Message and halt.
% ====================================================
Main    GETA    Arg0,Msg1
        SET     Arg1,0
        PUSHJ   $5,Sha256Message

        GETA    Arg0,Msg2
        SET     Arg1,3
        PUSHJ   $5,Sha256Message

        GETA    Arg0,Msg3
        SET     Arg1,56
        PUSHJ   $5,Sha256Message

        SET     $255,0
        TRAP    0,Halt,0

% ====================================================
% Sha256Message: Arg0 = message pointer, Arg1 = length in bytes.
% Resets Hstate from H0table, pads into PadBuf, folds every block
% through Sha256Block, then prints the digest.
% Locals: $0 = saved rJ, $1 = block index, $2 = block count; every call
% below uses hole $10, so $0..$2 come back from each call untouched.
% ====================================================
Sha256Message GET $0,rJ
        GETA    $4,H0table
        GETA    $5,Hstate
        SET     $6,0
ResetLp CMPU    $7,$6,8
        BNN     $7,ResetDone
        SLU     $8,$6,2
        LDTU    $9,$4,$8
        STTU    $9,$5,$8
        ADDU    $6,$6,1
        JMP     ResetLp
ResetDone

        PUSHJ   $10,Pad                 % Arg0, Arg1 already set by caller
        SET     $2,$10                  % block count

        SET     $1,0
BlockLp CMPU    $11,$1,$2
        BNN     $11,BlockDone
        GETA    $12,PadBuf
        SLU     $13,$1,6
        ADDU    Arg0,$12,$13
        PUSHJ   $10,Sha256Block
        ADDU    $1,$1,1
        JMP     BlockLp
BlockDone

        PUSHJ   $10,HexPrint
        PUT     rJ,$0
        POP     0,0

% ====================================================
% Pad: Arg0 = message pointer, Arg1 = length. Writes the padded message
% into PadBuf (0x80, zero fill, then the bit length as a big-endian
% octabyte) and returns the block count.
% ====================================================
Pad     GET     $0,rJ
        SET     $1,Arg0                 % src
        SET     $2,Arg1                 % length
        GETA    $3,PadBuf

        SET     $4,0
CopyLp  CMPU    $5,$4,$2
        BNN     $5,CopyDone
        LDBU    $6,$1,$4
        STBU    $6,$3,$4
        ADDU    $4,$4,1
        JMP     CopyLp
CopyDone
        SET     $6,#80
        STBU    $6,$3,$2

        ADDU    $7,$2,72                % length + 1 (marker) + 8 (length field) + 63
        PUT     rD,0
        DIVU    $8,$7,64                % block count
        SLU     $9,$8,6                 % padded length P = blocks*64

        ADDU    $4,$2,1
        SUBU    $10,$9,8                % P - 8
ZeroLp  CMPU    $5,$4,$10
        BNN     $5,ZeroDone
        STBU    Zero,$3,$4
        ADDU    $4,$4,1
        JMP     ZeroLp
ZeroDone
        SLU     $11,$2,3                % bit length = byte length * 8
        STOU    $11,$3,$10

        SET     $0,$8
        POP     1,0

% ====================================================
% Sha256Block: Arg0 = pointer to one 64-byte block. Expands the message
% schedule into WBuf, runs the 64-round compression, and folds the
% result into Hstate.
% ====================================================
Sha256Block SET $2,Arg0
        GETA    $3,WBuf

        SET     $12,0
WCopy   CMPU    $13,$12,16
        BNN     $13,WCopyDone
        SLU     $14,$12,2
        LDTU    $15,$2,$14
        STTU    $15,$3,$14
        ADDU    $12,$12,1
        JMP     WCopy
WCopyDone

        SET     $4,16
WExpand CMPU    $50,$4,64
        BNN     $50,WExpandDone

        SUBU    $51,$4,2
        SLU     $51,$51,2
        LDTU    $8,$3,$51
        SRU     $40,$8,17
        SLU     $41,$8,15
        OR      $40,$40,$41
        AND     $40,$40,Mask32
        SRU     $41,$8,19
        SLU     $42,$8,13
        OR      $41,$41,$42
        AND     $41,$41,Mask32
        SRU     $42,$8,10
        XOR     $60,$40,$41
        XOR     $60,$60,$42             % sigma1(W[t-2])

        SUBU    $51,$4,7
        SLU     $51,$51,2
        LDTU    $61,$3,$51              % W[t-7]

        SUBU    $51,$4,15
        SLU     $51,$51,2
        LDTU    $8,$3,$51
        SRU     $40,$8,7
        SLU     $41,$8,25
        OR      $40,$40,$41
        AND     $40,$40,Mask32
        SRU     $41,$8,18
        SLU     $42,$8,14
        OR      $41,$41,$42
        AND     $41,$41,Mask32
        SRU     $42,$8,3
        XOR     $62,$40,$41
        XOR     $62,$62,$42             % sigma0(W[t-15])

        SUBU    $51,$4,16
        SLU     $51,$51,2
        LDTU    $63,$3,$51              % W[t-16]

        ADDU    $64,$60,$61
        ADDU    $64,$64,$62
        ADDU    $64,$64,$63
        AND     $64,$64,Mask32

        SLU     $51,$4,2
        STTU    $64,$3,$51

        ADDU    $4,$4,1
        JMP     WExpand
WExpandDone

        GETA    $19,Hstate
        LDTU    $10,$19,0               % a
        LDTU    $11,$19,4               % b
        LDTU    $12,$19,8               % c
        LDTU    $13,$19,12              % d
        LDTU    $14,$19,16              % e
        LDTU    $15,$19,20              % f
        LDTU    $16,$19,24              % g
        LDTU    $17,$19,28              % h

        SET     $18,0
RoundLp CMPU    $50,$18,64
        BNN     $50,RoundDone

        SRU     $40,$14,6
        SLU     $41,$14,26
        OR      $40,$40,$41
        AND     $40,$40,Mask32
        SRU     $41,$14,11
        SLU     $42,$14,21
        OR      $41,$41,$42
        AND     $41,$41,Mask32
        SRU     $42,$14,25
        SLU     $43,$14,7
        OR      $42,$42,$43
        AND     $42,$42,Mask32
        XOR     $70,$40,$41
        XOR     $70,$70,$42             % Sigma1(e)

        PUT     rM,$14
        MUX     $71,$15,$16             % Ch(e,f,g)

        GETA    $52,Ktable
        SLU     $53,$18,2
        LDTU    $72,$52,$53             % K[t]
        LDTU    $73,$3,$53              % W[t]

        ADDU    $74,$17,$70
        ADDU    $74,$74,$71
        ADDU    $74,$74,$72
        ADDU    $74,$74,$73
        AND     $74,$74,Mask32          % T1

        SRU     $40,$10,2
        SLU     $41,$10,30
        OR      $40,$40,$41
        AND     $40,$40,Mask32
        SRU     $41,$10,13
        SLU     $42,$10,19
        OR      $41,$41,$42
        AND     $41,$41,Mask32
        SRU     $42,$10,22
        SLU     $43,$10,10
        OR      $42,$42,$43
        AND     $42,$42,Mask32
        XOR     $75,$40,$41
        XOR     $75,$75,$42             % Sigma0(a)

        AND     $77,$10,$11
        AND     $78,$10,$12
        AND     $79,$11,$12
        XOR     $76,$77,$78
        XOR     $76,$76,$79             % Maj(a,b,c)

        ADDU    $80,$75,$76
        AND     $80,$80,Mask32          % T2

        SET     $17,$16
        SET     $16,$15
        SET     $15,$14
        ADDU    $14,$13,$74
        AND     $14,$14,Mask32
        SET     $13,$12
        SET     $12,$11
        SET     $11,$10
        ADDU    $10,$74,$80
        AND     $10,$10,Mask32

        ADDU    $18,$18,1
        JMP     RoundLp
RoundDone

        LDTU    $90,$19,0
        ADDU    $90,$90,$10
        AND     $90,$90,Mask32
        STTU    $90,$19,0

        LDTU    $90,$19,4
        ADDU    $90,$90,$11
        AND     $90,$90,Mask32
        STTU    $90,$19,4

        LDTU    $90,$19,8
        ADDU    $90,$90,$12
        AND     $90,$90,Mask32
        STTU    $90,$19,8

        LDTU    $90,$19,12
        ADDU    $90,$90,$13
        AND     $90,$90,Mask32
        STTU    $90,$19,12

        LDTU    $90,$19,16
        ADDU    $90,$90,$14
        AND     $90,$90,Mask32
        STTU    $90,$19,16

        LDTU    $90,$19,20
        ADDU    $90,$90,$15
        AND     $90,$90,Mask32
        STTU    $90,$19,20

        LDTU    $90,$19,24
        ADDU    $90,$90,$16
        AND     $90,$90,Mask32
        STTU    $90,$19,24

        LDTU    $90,$19,28
        ADDU    $90,$90,$17
        AND     $90,$90,Mask32
        STTU    $90,$19,28

        POP     0,0

% ====================================================
% HexPrint: writes Hstate as 64 lowercase hex digits and a newline.
% ====================================================
HexPrint GETA  $1,Hstate
        GETA    $2,HexOut
        GETA    $3,HexDigits

        SET     $4,0
HPWord  CMPU    $5,$4,8
        BNN     $5,HPWordDone
        SLU     $6,$4,2
        LDTU    $7,$1,$6

        SET     $8,0
HPNib   CMPU    $9,$8,8
        BNN     $9,HPNibDone
        SET     $11,28
        SLU     $12,$8,2
        SUBU    $10,$11,$12
        SRU     $13,$7,$10
        AND     $13,$13,15
        LDBU    $14,$3,$13
        SLU     $15,$4,3
        ADDU    $15,$15,$8
        STBU    $14,$2,$15
        ADDU    $8,$8,1
        JMP     HPNib
HPNibDone
        ADDU    $4,$4,1
        JMP     HPWord
HPWordDone
        SET     $16,10
        STBU    $16,$2,64
        STBU    Zero,$2,65

        SET     $255,$2
        TRAP    0,Fputs,StdOut

        POP     0,0
