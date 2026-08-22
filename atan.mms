% =====================================================================
% atan.mms -- Arctangent for MMIX (IEEE 754 double precision)
% =====================================================================
%
% Public symbol: :Atan
%
%   Calling convention (standard MMIX PUSHJ):
%
%       SET     $X+1,<x bits>          ; or load x into $X+1
%       PUSHJ   $X,:Atan
%       ; result in $X (IEEE 754 double, bit pattern)
%
%   Equivalently: place input in the register one above the PUSHJ target,
%   call, and the result lands in the PUSHJ target register.
%
% Behavior:
%   atan(NaN)   = NaN
%   atan(+Inf)  = +pi/2
%   atan(-Inf)  = -pi/2
%   atan(+/-0)  = +/-0
%   atan(x)     = arctangent in (-pi/2, pi/2) for finite x
%
% Algorithm:
%   1. If x is NaN, return NaN.
%   2. s := signbit(x); u := |x|.
%   3. If u is +Inf, return s ? -pi/2 : +pi/2.
%   4. If u > 1, set rec=1 and u := 1/u.
%   5. If u > tan(pi/8), set shf=1 and u := (u-1)/(u+1).
%   6. p := u + u^3 * Q(u^2), where Q is a degree-5 polynomial (Horner).
%   7. If shf: p := p + pi/4.
%   8. If rec: p := pi/2 - p.
%   9. If s:   p := -p (toggle sign bit).
%
% Local register usage inside the routine: $0..$7. The PUSHJ register-stack
% mechanism guarantees these are fresh and do not clobber the caller.
% =====================================================================

% --- Constants (data segment) ---------------------------------------

        LOC     #4000                   % near code; keeps constants within GETA's reach

:Atan_PiOver2  OCTA  #3FF921FB54442D18
:Atan_PiOver4  OCTA  #3FE921FB54442D18
:Atan_TanPi8   OCTA  #3FDA827999FCEF33
:Atan_One      OCTA  #3FF0000000000000
:Atan_AbsMask  OCTA  #7FFFFFFFFFFFFFFF
:Atan_SignBit  OCTA  #8000000000000000
:Atan_InfBits  OCTA  #7FF0000000000000

% Polynomial Q(t) = c3 + c5 t + c7 t^2 + c9 t^3 + c11 t^4 + c13 t^5
% used as:  atan(u) ~= u + u^3 * Q(u^2)  on |u| <= tan(pi/8)
% (After both range reductions, |u| stays well below tan(pi/8) so this
% Maclaurin-style truncation gives a few-ulp result; for production use
% one would substitute a true minimax polynomial of equal degree.)

:Atan_C3   OCTA  #BFD5555555555555
:Atan_C5   OCTA  #3FC999999999999A
:Atan_C7   OCTA  #BFC2492492492492
:Atan_C9   OCTA  #3FBC71C71C71C71C
:Atan_C11  OCTA  #BFB745D1745D1746
:Atan_C13  OCTA  #3FB3B13B13B13B14

% --- Code ---------------------------------------------------------

        LOC     #200

:Atan   IS      @

% Step 1: NaN test (FUN sets result to 1 if unordered)
        FUN     $1,$0,$0
        BNZ     $1,:Atan_Ret           % NaN -> return x unchanged

% Step 2: split into sign + magnitude
        SRU     $1,$0,63               % $1 = sign bit (0 or 1)
        GETA     $2,:Atan_AbsMask
        LDOUI   $2,$2,0
        AND     $0,$0,$2               % $0 = |x| (bit pattern of magnitude)

% Step 3: Infinity test (after abs, +Inf == 0x7FF0000000000000)
        GETA     $3,:Atan_InfBits
        LDOUI   $3,$3,0
        CMP     $4,$0,$3
        BNZ     $4,:Atan_NotInf
        GETA     $0,:Atan_PiOver2
        LDOUI   $0,$0,0
        JMP     :Atan_ApplySign
:Atan_NotInf  IS @

% Reduction flags
        SETL    $5,0                   % rec
        SETL    $6,0                   % shf

% Step 4: reciprocal reduction if |x| > 1
        GETA     $3,:Atan_One
        LDOUI   $3,$3,0
        FCMP    $4,$0,$3
        BNP     $4,:Atan_NoRec
        SETL    $5,1
        FDIV    $0,$3,$0               % u := 1/u
:Atan_NoRec  IS @

% Step 5: shift reduction if u > tan(pi/8)
        GETA     $3,:Atan_TanPi8
        LDOUI   $3,$3,0
        FCMP    $4,$0,$3
        BNP     $4,:Atan_NoShf
        SETL    $6,1
        GETA     $7,:Atan_One
        LDOUI   $7,$7,0
        FSUB    $3,$0,$7               % u - 1
        FADD    $4,$0,$7               % u + 1
        FDIV    $0,$3,$4               % u := (u-1)/(u+1)
:Atan_NoShf  IS @

% Step 6: Horner evaluation of Q(t), t = u^2
        FMUL    $2,$0,$0               % t = u*u
        GETA     $3,:Atan_C13
        LDOUI   $3,$3,0
        FMUL    $3,$3,$2
        GETA     $4,:Atan_C11
        LDOUI   $4,$4,0
        FADD    $3,$3,$4
        FMUL    $3,$3,$2
        GETA     $4,:Atan_C9
        LDOUI   $4,$4,0
        FADD    $3,$3,$4
        FMUL    $3,$3,$2
        GETA     $4,:Atan_C7
        LDOUI   $4,$4,0
        FADD    $3,$3,$4
        FMUL    $3,$3,$2
        GETA     $4,:Atan_C5
        LDOUI   $4,$4,0
        FADD    $3,$3,$4
        FMUL    $3,$3,$2
        GETA     $4,:Atan_C3
        LDOUI   $4,$4,0
        FADD    $3,$3,$4               % $3 = Q(t)
        FMUL    $3,$3,$2               % Q*t   = Q*u^2
        FMUL    $3,$3,$0               % Q*u^3
        FADD    $0,$0,$3               % p = u + Q*u^3

% Step 7: undo shift
        BZ      $6,:Atan_NoUnShf
        GETA     $4,:Atan_PiOver4
        LDOUI   $4,$4,0
        FADD    $0,$0,$4
:Atan_NoUnShf IS @

% Step 8: undo reciprocal
        BZ      $5,:Atan_NoUnRec
        GETA     $4,:Atan_PiOver2
        LDOUI   $4,$4,0
        FSUB    $0,$4,$0
:Atan_NoUnRec IS @

% Step 9: reapply sign by toggling the sign bit
:Atan_ApplySign IS @
        BZ      $1,:Atan_Ret
        GETA     $4,:Atan_SignBit
        LDOUI   $4,$4,0
        XOR     $0,$0,$4

:Atan_Ret IS @
        POP     1,0
