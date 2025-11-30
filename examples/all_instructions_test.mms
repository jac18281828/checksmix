% MMIX Comprehensive Instruction Test
% This program tests all major instruction families
% It validates itself - if it completes without error, all tests passed

        LOC     #100

% Test counter and expected values
Expect  IS      $1      % Expected value
Result  IS      $2      % Actual result
FailNum IS      $3      % Failed test number
TestNum IS      $4      % Current test number 
Temp    IS      $254    % Temporary register (canonical t register)
Zero    IS      $255    % ZERO register

Main    SET     TestNum,0       % Initialize test counter

% ========================================
% Test 1: SET and immediate load instructions
% ========================================
Test1   ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     Result,#0123456789ABCDEF
        SET     Expect,#0123456789ABCDEF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test2
        JMP     TestFail

% ========================================
% Test 2: SETL, SETH, SETMH, SETML
% ========================================
Test2   ADDUI   TestNum,TestNum,1       % Increment test counter
        SETH    Result,#0123
        SETMH   Result,#4567
        SETML   Result,#89AB
        SETL    Result,#CDEF
        SET     Expect,#0123456789ABCDEF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test3
        JMP     TestFail

% ========================================
% Test 3: Bitwise OR operations
% ========================================
Test3   ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#FF00
        SET     $11,#00FF
        OR      Result,$10,$11
        SET     Expect,#FFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test4
        JMP     TestFail

% ========================================
% Test 4: Bitwise AND operations
% ========================================
Test4   ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#FF0F
        SET     $11,#0FFF
        AND     Result,$10,$11
        SET     Expect,#0F0F
        CMP     Temp,Result,Expect
        PBZ     Temp,Test5
        JMP     TestFail

% ========================================
% Test 5: Bitwise XOR operations
% ========================================
Test5   ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#FFFF
        SET     $11,#F0F0
        XOR     Result,$10,$11
        SET     Expect,#0F0F
        CMP     Temp,Result,Expect
        PBZ     Temp,Test6
        JMP     TestFail

% ========================================
% Test 6: ANDN operation
% ========================================
Test6   ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#FFFF
        SET     $11,#0F0F
        ANDN    Result,$10,$11
        SET     Expect,#F0F0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test7
        JMP     TestFail

% ========================================
% Test 7: NOR operation
% ========================================
Test7   ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#00
        SET     $11,#00
        NOR     Result,$10,$11
        SET     Expect,#0
        SUBUI   Expect,Expect,1 % All 1s (-1)
        CMP     Temp,Result,Expect
        PBZ     Temp,Test8
        JMP     TestFail

% ========================================
% Test 8: NAND operation
% ========================================
Test8   ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#FFFF
        SET     $11,#FFFF
        NAND    Result,$10,$11
        SET     Expect,#0
        SUBUI   Expect,Expect,1 % All 1s
        SET     $12,#FFFF
        XOR     Expect,Expect,$12       % Should give ~#FFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test9
        JMP     TestFail

% ========================================
% Test 9: NXOR operation
% ========================================
Test9   ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#F0F0
        SET     $11,#F0F0
        NXOR    Result,$10,$11
        SET     Expect,#0
        SUBUI   Expect,Expect,1 % All 1s
        CMP     Temp,Result,Expect
        PBZ     Temp,Test10
        JMP     TestFail

% ========================================
% Test 10: ADDU - Unsigned addition
% ========================================
Test10  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,100
        SET     $11,50
        ADDU    Result,$10,$11
        SET     Expect,150
        CMP     Temp,Result,Expect
        PBZ     Temp,Test11
        JMP     TestFail

% ========================================
% Test 11: SUBU - Unsigned subtraction
% ========================================
Test11  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,100
        SET     $11,30
        SUBU    Result,$10,$11
        SET     Expect,70
        CMP     Temp,Result,Expect
        PBZ     Temp,Test12
        JMP     TestFail

% ========================================
% Test 12: 2ADDU - Times 2 and add
% ========================================
Test12  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,10
        SET     $11,5
        2ADDU   Result,$10,$11
        SET     Expect,25       % 10*2 + 5 = 25
        CMP     Temp,Result,Expect
        PBZ     Temp,Test13
        JMP     TestFail

% ========================================
% Test 13: 4ADDU - Times 4 and add
% ========================================
Test13  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,10
        SET     $11,5
        4ADDU   Result,$10,$11
        SET     Expect,45       % 10*4 + 5 = 45
        CMP     Temp,Result,Expect
        PBZ     Temp,Test14
        JMP     TestFail

% ========================================
% Test 14: 8ADDU - Times 8 and add
% ========================================
Test14  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,10
        SET     $11,5
        8ADDU   Result,$10,$11
        SET     Expect,85       % 10*8 + 5 = 85
        CMP     Temp,Result,Expect
        PBZ     Temp,Test15
        JMP     TestFail

% ========================================
% Test 15: 16ADDU - Times 16 and add
% ========================================
Test15  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,10
        SET     $11,5
        16ADDU  Result,$10,$11
        SET     Expect,165      % 10*16 + 5 = 165
        CMP     Temp,Result,Expect
        PBZ     Temp,Test16
        JMP     TestFail

% ========================================
% Test 16: CMP - Signed comparison (less than)
% ========================================
Test16  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,5
        SET     $11,10
        CMP     Result,$10,$11
        SET     Expect,#0
        SUBUI   Expect,Expect,1        % -1 (less than)
        CMP     Temp,Result,Expect
        PBZ     Temp,Test17
        JMP     TestFail

% ========================================
% Test 17: CMP - Signed comparison (equal)
% ========================================
Test17  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,42
        SET     $11,42
        CMP     Result,$10,$11
        SET     Expect,0        % Equal
        CMP     Temp,Result,Expect
        PBZ     Temp,Test18
        JMP     TestFail

% ========================================
% Test 18: CMP - Signed comparison (greater than)
% ========================================
Test18  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,100
        SET     $11,50
        CMP     Result,$10,$11
        SET     Expect,1        % Greater than
        CMP     Temp,Result,Expect
        PBZ     Temp,Test19
        JMP     TestFail

% ========================================
% Test 19: CMPU - Unsigned comparison
% ========================================
Test19  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,100
        SET     $11,50
        CMPU    Result,$10,$11
        SET     Expect,1        % Greater than
        CMP     Temp,Result,Expect
        PBZ     Temp,Test20
        JMP     TestFail

% ========================================
% Test 20: Memory load/store - BYTE
% ========================================
Test20  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $20,#100        % Address
        SET     $21,#42         % Value to store
        STB     $21,$20,Zero    % Store byte
        LDB     Result,$20,Zero % Load it back
        SET     Expect,#42
        CMP     Temp,Result,Expect
        PBZ     Temp,Test21
        JMP     TestFail

% ========================================
% Test 21: Memory load/store - WYDE
% ========================================
Test21  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $20,#200        % Address
        SET     $21,#ABCD       % Value to store
        STW     $21,$20,Zero    % Store wyde
        LDWU    Result,$20,Zero % Load it back (unsigned)
        SET     Expect,#ABCD
        CMP     Temp,Result,Expect
        PBZ     Temp,Test22
        JMP     TestFail

% ========================================
% Test 22: Memory load/store - TETRA
% ========================================
Test22  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $20,#300        % Address
        SET     $21,#12345678   % Value to store
        STT     $21,$20,Zero    % Store tetra
        LDT     Result,$20,Zero % Load it back
        SET     Expect,#12345678
        CMP     Temp,Result,Expect
        PBZ     Temp,Test23
        JMP     TestFail

% ========================================
% Test 23: Memory load/store - OCTA
% ========================================
Test23  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $20,#400        % Address (8-byte aligned)
        SET     $21,#123456789ABCDEF
        STO     $21,$20,Zero    % Store octa
        LDO     Result,$20,Zero % Load it back
        SET     Expect,#123456789ABCDEF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test24
        JMP     TestFail

% ========================================
% Test 24: ODIF - Octa difference
% ========================================
Test24  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#FF
        SET     $11,#0F
        ODIF    Result,$10,$11
        SET     Expect,#F0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test25
        JMP     TestFail

% ========================================
% Test 25: SADD - Sideways add
% ========================================
Test25  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#0101010101010101
        SADD    Result,$10,Zero
        SET     Expect,8        % Eight 1-bits
        CMP     Temp,Result,Expect
        PBZ     Temp,Test26
        JMP     TestFail

% ========================================
% Test 26: SL - Shift left
% ========================================
Test26  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#1
        SET     $11,4
        SL      Result,$10,$11
        SET     Expect,#10
        CMP     Temp,Result,Expect
        PBZ     Temp,Test27
        JMP     TestFail

% ========================================
% Test 27: SR - Shift right
% ========================================
Test27  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#80
        SET     $11,4
        SR      Result,$10,$11
        SET     Expect,#8
        CMP     Temp,Result,Expect
        PBZ     Temp,Test28
        JMP     TestFail

% ========================================
% COMPREHENSIVE ADD/SUB FAMILY TESTS
% Testing all arithmetic instructions (0x20-0x2F)
% ========================================

% ========================================
% Test 28: ADD - Signed addition (no overflow)
% ========================================
Test28  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,100
        SET     $11,50
        ADD     Result,$10,$11
        SET     Expect,150
        CMP     Temp,Result,Expect
        PBZ     Temp,Test29
        JMP     TestFail

% ========================================
% Test 29: ADDI - Add immediate (no overflow)
% ========================================
Test29  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,100
        ADDI    Result,$10,75
        SET     Expect,175
        CMP     Temp,Result,Expect
        PBZ     Temp,Test30
        JMP     TestFail

% ========================================
% Test 30: ADDU - Unsigned addition
% ========================================
Test30  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#FFFFFFFFFFFFFFFF
        SET     $11,1
        ADDU    Result,$10,$11
        SET     Expect,0        % Wraps around
        CMP     Temp,Result,Expect
        PBZ     Temp,Test31
        JMP     TestFail

% ========================================
% Test 31: ADDUI - Add unsigned immediate
% ========================================
Test31  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,#FFFFFFFFFFFFFF00
        ADDUI   Result,$10,#FF
        SET     Expect,#FFFFFFFFFFFFFFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test32
        JMP     TestFail

% ========================================
% Test 32: SUB - Signed subtraction
% ========================================
Test32  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,100
        SET     $11,30
        SUB     Result,$10,$11
        SET     Expect,70
        CMP     Temp,Result,Expect
        PBZ     Temp,Test33
        JMP     TestFail

% ========================================
% Test 33: SUBI - Subtract immediate
% ========================================
Test33  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,200
        SUBI    Result,$10,50
        SET     Expect,150
        CMP     Temp,Result,Expect
        PBZ     Temp,Test34
        JMP     TestFail

% ========================================
% Test 34: SUBU - Unsigned subtraction
% ========================================
Test34  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,10
        SET     $11,20
        SUBU    Result,$10,$11
        SET     Expect,#FFFFFFFFFFFFFFF6  % -10 as unsigned
        CMP     Temp,Result,Expect
        PBZ     Temp,Test35
        JMP     TestFail

% ========================================
% Test 35: SUBUI - Subtract unsigned immediate
% ========================================
Test35  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,50
        SUBUI   Result,$10,100
        SET     Expect,#FFFFFFFFFFFFFFCE  % -50 as unsigned
        CMP     Temp,Result,Expect
        PBZ     Temp,Test36
        JMP     TestFail

% ========================================
% Test 36: 2ADDU - Times 2 and add (register)
% ========================================
Test36  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,25
        SET     $11,10
        2ADDU   Result,$10,$11
        SET     Expect,60       % 25*2 + 10 = 60
        CMP     Temp,Result,Expect
        PBZ     Temp,Test37
        JMP     TestFail

% ========================================
% Test 37: 2ADDUI - Times 2 and add (immediate)
% ========================================
Test37  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,30
        2ADDUI  Result,$10,15
        SET     Expect,75       % 30*2 + 15 = 75
        CMP     Temp,Result,Expect
        PBZ     Temp,Test38
        JMP     TestFail

% ========================================
% Test 38: 4ADDU - Times 4 and add (register)
% ========================================
Test38  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,20
        SET     $11,12
        4ADDU   Result,$10,$11
        SET     Expect,92       % 20*4 + 12 = 92
        CMP     Temp,Result,Expect
        PBZ     Temp,Test39
        JMP     TestFail

% ========================================
% Test 39: 4ADDUI - Times 4 and add (immediate)
% ========================================
Test39  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,15
        4ADDUI  Result,$10,8
        SET     Expect,68       % 15*4 + 8 = 68
        CMP     Temp,Result,Expect
        PBZ     Temp,Test40
        JMP     TestFail

% ========================================
% Test 40: 8ADDU - Times 8 and add (register)
% ========================================
Test40  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,12
        SET     $11,7
        8ADDU   Result,$10,$11
        SET     Expect,103      % 12*8 + 7 = 103
        CMP     Temp,Result,Expect
        PBZ     Temp,Test41
        JMP     TestFail

% ========================================
% Test 41: 8ADDUI - Times 8 and add (immediate)
% ========================================
Test41  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,9
        8ADDUI  Result,$10,6
        SET     Expect,78       % 9*8 + 6 = 78
        CMP     Temp,Result,Expect
        PBZ     Temp,Test42
        JMP     TestFail

% ========================================
% Test 42: 16ADDU - Times 16 and add (register)
% ========================================
Test42  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,8
        SET     $11,4
        16ADDU  Result,$10,$11
        SET     Expect,132      % 8*16 + 4 = 132
        CMP     Temp,Result,Expect
        PBZ     Temp,Test43
        JMP     TestFail

% ========================================
% Test 43: 16ADDUI - Times 16 and add (immediate)
% ========================================
Test43  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,7
        16ADDUI Result,$10,3
        SET     Expect,115      % 7*16 + 3 = 115
        CMP     Temp,Result,Expect
        PBZ     Temp,TestPass
        JMP     TestFail

% ========================================
% Test 44: $255 is ZERO register check
% ========================================
Test44  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     $10,12345
        ADD     Result,$10,Zero
        SET     Expect,12345
        CMP     Temp,Result,Expect
        PBZ     Temp,Test45
        JMP     TestFail

% ========================================
% Test 45: GETA - Get address (forward reference)
% ========================================
Test45  ADDUI   TestNum,TestNum,1       % Increment test counter
        GETA    Result,LocalData  % Get address of LocalData
        LDO     $10,Result,Zero   % Load the octa at that address (offset 0)
        SET     Expect,#DEADBEEFCAFEBABE
        CMP     Temp,$10,Expect
        PBZ     Temp,Test46
        JMP     TestFail

% ========================================
% Test 46: GETA - Get address (backward reference)
% ========================================
Test46  ADDUI   TestNum,TestNum,1       % Increment test counter
        GETA    Result,Test1      % Get address of Test1 (backward)
        SET     Expect,Test1      % SET computes full 64-bit address
        CMP     Temp,Result,Expect
        PBZ     Temp,Test47
        JMP     TestFail

% ========================================
% Test 47: JMP - Unconditional jump forward
% ========================================
Test47  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     Result,#BAD       % This should be skipped
        JMP     Test47Skip
        SET     Result,#DEAD      % This should also be skipped
Test47Skip      SET     Expect,#BAD
        CMP     Temp,Result,Expect
        PBZ     Temp,Test48
        JMP     TestFail

% ========================================
% Test 48: JMP - Unconditional jump backward
% ========================================
Test48  ADDUI   TestNum,TestNum,1       % Increment test counter
        JMP     Test48Start
Test48Target    SET     Result,#C0DE
        JMP     Test48End
Test48Start     SET     Result,#BAD
        JMP     Test48Target
Test48End       SET     Expect,#C0DE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test49
        JMP     TestFail

% ========================================
% Test 49: JMP - Large forward offset
% ========================================
Test49  ADDUI   TestNum,TestNum,1       % Increment test counter
        SET     Result,0
        JMP     Test49Far
        JMP     TestFail        % Should not reach here

% ========================================
% Test 50: JMP - Large backward offset  
% ========================================
Test50  ADDUI   TestNum,TestNum,1       % Increment test counter
        JMP     Test50Start
Test50End       SET     Expect,#BABE
        CMP     Temp,Result,Expect
        PBZ     Temp,TestPass
        JMP     TestFail
Test50Start     SET     Result,#BABE
        JMP     Test50End

% ========================================
% All tests passed!
% ========================================
TestPass        SET     $0,PassMsg
        TRAP    0,Fputs,StdOut
        SETL    Result,#FFFF    % Success marker
        TRAP    0,Halt,0        % Halt successfully

% ========================================
% Test failed
% ========================================
TestFail        SET     $0,FailMsg
        TRAP    0,Fputs,StdOut
        SET     Result,#DEAD    % Failure marker
        OR      FailNum,TestNum,Zero    % Copy TestNum to FailNum
        TRAP    0,Halt,1        % Halt with error

% ========================================
% debug subroutine
% ========================================

% ========================================
% Local data for GETA test (must be in code segment, near the code)
% ========================================
        OCTA    0
LocalData
        OCTA    #DEADBEEFCAFEBABE

% ========================================
% Test 49 Far Target - Located far away to test large JMP offset
% ========================================
        LOC     #10000          % Jump to a far location (64KB away)
Test49Far       SET     Result,#CAFE
        SET     Expect,#CAFE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test50
        JMP     TestFail

% ========================================
% Data section - Messages
% ========================================
        LOC     Data_Segment
PassMsg BYTE    "All tests passed!",10,0
FailMsg BYTE    "Test failed!",10,0
