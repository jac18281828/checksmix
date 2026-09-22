% greg_base_address.mms - the two-operand memory form's base-address
% search: a GREG holding a nonzero value is a base address, and a pure
% second operand within 0-255 bytes of it resolves against it in Y and Z.
% Every register this program touches (Expect, Result, Temp) is written
% before it is ever read.

Expect  IS      $1
Result  IS      $2
Temp    IS      $3

        LOC     #100
Base    GREG    @               % the base address the checks resolve against
DataA   OCTA    123456
DataB   OCTA    99
DataAddr OCTA   0
Sub     SET     Result,1        % the two-operand GO's target, also base-relative
        SET     Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Pass
        JMP     Fail
PassMsg BYTE    "All tests passed!",10,0
FailMsg BYTE    "Test failed!",10,0

        LOC     #200
Main    SET     $10,DataA
        STO     $10,DataAddr    % the two-operand store form
        LDO     Result,DataAddr % the two-operand load form
        SET     Expect,DataA
        CMP     Temp,Result,Expect
        PBZ     Temp,Check2
        JMP     Fail

Check2  LDO     Result,DataB    % a second base-relative load
        SET     Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Check3
        JMP     Fail

Check3  GO      $9,Sub          % the two-operand form of GO
        JMP     Fail            % unreached: Sub falls through to Pass

Pass    SETI    $255,PassMsg
        TRAP    0,Fputs,StdOut
        SETI    $255,0
        TRAP    0,Halt,0

Fail    SETI    $255,FailMsg
        TRAP    0,Fputs,StdOut
        SETI    $255,1
        TRAP    0,Halt,0
