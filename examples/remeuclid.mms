MOD IS 100

	LOC #100
% Comprehensive tests for RemEuclid
% Test results in $20 (bit field: bit N = test N passed)
Main	SETI	$20,0		% Test results accumulator

	% Test 1: Positive in range [0, MOD-1]: 42 % 100 = 42
	SETI	$0,42
	SETI	$1,MOD
	PUSHJ	$2,RemEuclid
	CMPI	$3,$0,42
	BNZ	$3,Test2
	ORI	$20,$20,#01

	% Test 2: Positive >= MOD: 142 % 100 = 42
Test2	SETI	$0,142
	SETI	$1,MOD
	PUSHJ	$2,RemEuclid
	CMPI	$3,$0,42
	BNZ	$3,Test3
	ORI	$20,$20,#02

	% Test 3: Negative in range: -58 % 100 = 42
Test3	SETI	$0,-58
	SETI	$1,MOD
	PUSHJ	$2,RemEuclid
	CMPI	$3,$0,42
	BNZ	$3,Test4
	ORI	$20,$20,#04

	% Test 4: Large negative: -194 % 100 = 6
Test4	SETI	$0,-194
	SETI	$1,MOD
	PUSHJ	$2,RemEuclid
	CMPI	$3,$0,6
	BNZ	$3,Test5
	ORI	$20,$20,#08

	% Test 5: Zero: 0 % 100 = 0
Test5	SETI	$0,0
	SETI	$1,MOD
	PUSHJ	$2,RemEuclid
	CMPI	$3,$0,0
	BNZ	$3,Test6
	ORI	$20,$20,#10

	% Test 6: Exactly MOD: 100 % 100 = 0
Test6	SETI	$0,100
	SETI	$1,MOD
	PUSHJ	$2,RemEuclid
	CMPI	$3,$0,0
	BNZ	$3,Test7
	ORI	$20,$20,#20

	% Test 7: Negative MOD: -100 % 100 = 0
Test7	SETI	$0,-100
	SETI	$1,MOD
	PUSHJ	$2,RemEuclid
	CMPI	$3,$0,0
	BNZ	$3,Test8
	ORI	$20,$20,#40

	% Test 8: Edge case: -1 % 100 = 99
Test8	SETI	$0,-1
	SETI	$1,MOD
	PUSHJ	$2,RemEuclid
	CMPI	$3,$0,99
	BNZ	$3,Done
	ORI	$20,$20,#80

Done	TRAP	0,Halt,0


% ------------------------------------------------------------
% RemEuclid: $0 := ($0 rem_euclid $1), with $1 > 0
% Input: $0 = dividend, $1 = divisor (must be > 0)
% Output: $0 = remainder in range [0, $1)
% Uses: $2, $3
% ------------------------------------------------------------
RemEuclid  DIV     $2, $0, $1        % q = trunc(a/m)
           MUL     $3, $2, $1        % q*m
           SUB     $0, $0, $3        % r = a - q*m  (can be negative)
           BNN     $0, RemDone       % if r >= 0, we're done
           ADD     $0, $0, $1        % r += m
RemDone    POP     1,0
