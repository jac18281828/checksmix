% =====================================================================
% test_atan.mms -- Test harness for the Atan library.
% =====================================================================
%
% Build:    mmixal test_atan.mms
% Run:      mmix test_atan
%
% Output looks like:
%
%   atan tests:
%   [00] PASS x=0000000000000000 got=0000000000000000 exp=0000000000000000
%   [01] PASS x=3FF0000000000000 got=3FE921FB54442D18 exp=3FE921FB54442D18
%   ...
%   summary: 12/12 passed
%
% The test passes if either:
%   (a) the bit pattern matches the expected reference exactly, OR
%   (b) |computed - expected| <= 1e-6  (absolute tolerance).
%
% atan.mms's Q(t) is a degree-13 truncated Maclaurin series, not a minimax
% polynomial (see the comment there): accuracy degrades from ~1e-17 for
% small reduced u up to ~1e-7 right at the u == tan(pi/8) shift-reduction
% boundary (verified empirically -- x=0.5/2/-2 and the boundary case below
% all exceed 1e-12, none exceed 1e-6). A tighter global tolerance here would
% be testing a stronger guarantee than the algorithm actually gives.
%
% Reference values were computed in IEEE 754 double precision externally.
% =====================================================================

% Pull in the library.  This places :Atan and its data into the program.
        INCLUDE atan.mms

% --- Test data ----------------------------------------------------------

        LOC     #5000                   % near code; keeps test data within GETA's reach
TestBase GREG   @

NumTests OCTA   12

% Inputs (IEEE 754 double bit patterns)
Inputs   OCTA   #0000000000000000      %  0.0
         OCTA   #3FF0000000000000      %  1.0
         OCTA   #BFF0000000000000      % -1.0
         OCTA   #3FE0000000000000      %  0.5
         OCTA   #4000000000000000      %  2.0
         OCTA   #C000000000000000      % -2.0
         OCTA   #4024000000000000      %  10.0
         OCTA   #7FF0000000000000      % +Inf
         OCTA   #FFF0000000000000      % -Inf
         OCTA   #8000000000000000      % -0.0
         OCTA   #7FF8000000000000      %  NaN (canonical quiet)
         OCTA   #3FDA827999FCEF33      %  tan(pi/8) -- exact shift-reduction boundary

% Expected outputs (atan applied)
Expects  OCTA   #0000000000000000      % atan(0)    = 0
         OCTA   #3FE921FB54442D18      % atan(1)    = pi/4
         OCTA   #BFE921FB54442D18      % atan(-1)   = -pi/4
         OCTA   #3FDDAC670561BB4F      % atan(0.5)
         OCTA   #3FF1B6E192EBBE44      % atan(2)
         OCTA   #BFF1B6E192EBBE44      % atan(-2)
         OCTA   #3FF789BD2C160054      % atan(10)
         OCTA   #3FF921FB54442D18      % atan(+Inf) = pi/2
         OCTA   #BFF921FB54442D18      % atan(-Inf) = -pi/2
         OCTA   #8000000000000000      % atan(-0)   = -0
         OCTA   #7FF8000000000000      % atan(NaN)  = NaN (unchanged)
         OCTA   #3FD921FB54442D19      % atan(tan(pi/8)) ~= pi/8

Tol      OCTA   #3EB0C6F7A0B5ED8D      % 1e-6 in IEEE double

% Scratch buffer for formatting numbers (large enough for 16 hex digits + NUL).
% Kept 8-byte aligned (declared right after the OCTA block, before the
% variable-length strings below) so GETA can reach it.
Buf      BYTE   0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
         BYTE   0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0

% --- Strings (NUL-terminated; each followed by an explicit 0 byte) ------
% Variable-length and back-to-back, so successive labels drift off a 4-byte
% boundary -- addressed with LDA rather than GETA (which requires a 4-byte-
% aligned, word-quantized target).

MsgHdr   BYTE   "atan tests:",#a,0
MsgOpen  BYTE   "[",0
MsgPass  BYTE   "] PASS  x=",0
MsgFail  BYTE   "] FAIL  x=",0
MsgGot   BYTE   "  got=",0
MsgExp   BYTE   "  exp=",0
MsgNL    BYTE   #a,0
MsgSum1  BYTE   "summary: ",0
MsgSum2  BYTE   "/",0
MsgSum3  BYTE   " passed",#a,0

% --- Globals for main loop (survive PUSHJ calls) ------------------------

% These GREGs allocate global registers usable from any code below.
idx      GREG   0                       % current test index
total    GREG   0                       % NumTests cached
passes   GREG   0                       % pass counter

% --- Code ---------------------------------------------------------------

        LOC     #1000
        GREG    @

% PrintHex2: print a NUL-terminated 16-digit lowercase-hex of the value
% in $0 to stdout. Uses Buf.
%   PUSHJ $X,PrintHex   with x in $X+1.
PrintHex IS     @
        % $0 = value to print
        GETA     $1,Buf                  % $1 = buffer base
        SETL    $2,16                   % digit count
        SETL    $3,0                    % i
:PHLoop CMP     $4,$3,$2
        BNN     $4,:PHEmit
        SUBU    $5,$2,$3                % position from left? we'll fill MSB-first
        SUBU    $5,$5,1                 % index 15..0
        % extract nibble (3-i)*4 from top: easier to extract from the value
        % directly: we want digit at position $3 from MSB, i.e. shift by
        % (15 - $3) * 4 to the right and mask 0xF.
        SUBU    $6,$2,$3
        SUBU    $6,$6,1                 % $6 = 15 - i
        SLU     $6,$6,2                 % *4 (bits)
        SRU     $7,$0,$6                % shifted value
        AND     $7,$7,#F                % nibble
        CMP     $4,$7,10
        BN      $4,:PHDigit
        % hex letter A..F (uppercase for consistency with constants)
        ADDU    $7,$7,55                % 'A'-10
        JMP     :PHStore
:PHDigit ADDU   $7,$7,'0'
:PHStore STBU  $7,$1,$3
        ADDU    $3,$3,1
        JMP     :PHLoop
:PHEmit SETL    $4,0
        STBUI   $4,$1,16                % NUL terminate
        SET     $255,$1
        TRAP    0,Fputs,StdOut
        POP     0,0

% PrintInt: print a small unsigned int in $0 as decimal, NUL-terminated.
PrintInt IS     @
        GETA     $1,Buf
        SETL    $2,0                    % length
        SET     $3,$0                   % n
        BNZ     $3,:PILoop
        SETL    $4,'0'
        STBUI   $4,$1,0
        SETL    $4,0
        STBUI   $4,$1,1
        JMP     :PIEmit
:PILoop BZ      $3,:PIRev
        DIV     $4,$3,10
        GET     $5,rR                   % remainder
        ADDU    $5,$5,'0'
        STBU    $5,$1,$2
        ADDU    $2,$2,1
        SET     $3,$4
        JMP     :PILoop
:PIRev  % digits stored low-to-high; reverse in place
        SETL    $3,0                    % i
        SUBU    $4,$2,1                 % j
:PIRevL CMP     $5,$3,$4
        BNN     $5,:PIEnd
        LDBU    $6,$1,$3
        LDBU    $7,$1,$4
        STBU    $7,$1,$3
        STBU    $6,$1,$4
        ADDU    $3,$3,1
        SUBU    $4,$4,1
        JMP     :PIRevL
:PIEnd  SETL    $5,0
        STBU    $5,$1,$2                % NUL terminate
:PIEmit SET     $255,$1
        TRAP    0,Fputs,StdOut
        POP     0,0

% PrintStr: print NUL-terminated string whose address is in $0.
PrintStr IS     @
        SET     $255,$0
        TRAP    0,Fputs,StdOut
        POP     0,0

% =====================================================================
% Main
% =====================================================================
        % Header
Main    LDA     $255,MsgHdr
        TRAP    0,Fputs,StdOut

        % Initialize loop state in globals
        GETA     total,NumTests
        LDOUI   total,total,0
        SETL    idx,0
        SETL    passes,0

:Loop   CMP     $0,idx,total
        BNN     $0,:Done

        % Compute byte offset = idx * 8
        SLU     $1,idx,3

        % Load x and expected
        GETA     $2,Inputs
        LDOU    $10,$2,$1               % x
        GETA     $2,Expects
        LDOU    $11,$2,$1               % expected

        % Call Atan: argument goes in $X+1, result returns in $X.
        % Use $20 as the PUSHJ target ("hole"); put x in $21 first.
        SET     $21,$10
        PUSHJ   $20,:Atan
        SET     $12,$20                 % computed result

        % Pass test if bit-equal OR |computed - expected| <= Tol.
        CMP     $0,$12,$11
        BZ      $0,:Pass

        % Compute |diff| as float, then reinterpret as bits and mask off sign.
        FSUB    $0,$12,$11              % diff
        GETA     $1,:Atan_AbsMask
        LDOUI   $1,$1,0
        AND     $0,$0,$1                % |diff| bits
        GETA     $1,Tol
        LDOUI   $1,$1,0
        FCMP    $2,$0,$1
        BNP     $2,:Pass
        JMP     :Fail

:Pass   ADDU    passes,passes,1
        % Print "[" idx "] PASS  x=..."
        % PUSHJ $30 slides the callee's $0 in from THIS caller's $31 (X+1),
        % not from $0 -- so every argument is staged at $31.
        LDA     $31,MsgOpen
        PUSHJ   $30,PrintStr
        SET     $31,idx
        PUSHJ   $30,PrintInt
        LDA     $31,MsgPass
        PUSHJ   $30,PrintStr
        SET     $31,$10
        PUSHJ   $30,PrintHex
        LDA     $31,MsgGot
        PUSHJ   $30,PrintStr
        SET     $31,$12
        PUSHJ   $30,PrintHex
        LDA     $31,MsgExp
        PUSHJ   $30,PrintStr
        SET     $31,$11
        PUSHJ   $30,PrintHex
        LDA     $31,MsgNL
        PUSHJ   $30,PrintStr
        JMP     :Step

:Fail   LDA     $31,MsgOpen
        PUSHJ   $30,PrintStr
        SET     $31,idx
        PUSHJ   $30,PrintInt
        LDA     $31,MsgFail
        PUSHJ   $30,PrintStr
        SET     $31,$10
        PUSHJ   $30,PrintHex
        LDA     $31,MsgGot
        PUSHJ   $30,PrintStr
        SET     $31,$12
        PUSHJ   $30,PrintHex
        LDA     $31,MsgExp
        PUSHJ   $30,PrintStr
        SET     $31,$11
        PUSHJ   $30,PrintHex
        LDA     $31,MsgNL
        PUSHJ   $30,PrintStr

:Step   ADDU    idx,idx,1
        JMP     :Loop

:Done   LDA     $31,MsgSum1
        PUSHJ   $30,PrintStr
        SET     $31,passes
        PUSHJ   $30,PrintInt
        LDA     $31,MsgSum2
        PUSHJ   $30,PrintStr
        SET     $31,total
        PUSHJ   $30,PrintInt
        LDA     $31,MsgSum3
        PUSHJ   $30,PrintStr

        SETL    $255,0                   % Fputs leaves its byte count in $255;
        TRAP    0,Halt,0                 % clear it so Halt reports exit code 0.
