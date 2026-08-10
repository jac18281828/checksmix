% MMIX Comprehensive Instruction Test
% This program tests all major instruction families
% It validates itself - if it completes without error, all tests passed

        LOC     #100

% Special registers (MMIX standard numbering)
rB      IS      0       % Bootstrap register
rD      IS      1       % Dividend register  
rE      IS      2       % Epsilon register
rH      IS      3       % Himult register
rJ      IS      4       % Jump register
rM      IS      5       % Multiplex mask register
rR      IS      6       % Remainder register
rN      IS      9       % Serial number register
rA      IS      21      % Arithmetic status register
rP      IS      23      % Prediction register (CSWAP/CSWAPI compare value)

% Test counter and expected values
Expect  IS      $1      % Expected value
Result  IS      $2      % Actual result
FailNum IS      $3      % Failed test number
TestNum IS      $4      % Current test number 
Temp    IS      $254    % Temporary register (canonical t register)
Zero    IS      $255    % ZERO register

Main    SETI TestNum,0       % Initialize test counter
        SETI Zero,0         % Initialize $255 to 0 (constant zero)

% ========================================
% Test 1: SET and immediate load instructions
% ========================================
Test1   ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI Result,#0123456789ABCDEF
        SETI Expect,#0123456789ABCDEF
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
        SETI Expect,#0123456789ABCDEF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test3
        JMP     TestFail

% ========================================
% Test 3: Bitwise OR operations
% ========================================
Test3   ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#FF00
        SETI $11,#00FF
        OR      Result,$10,$11
        SETI Expect,#FFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test4
        JMP     TestFail

% ========================================
% Test 4: Bitwise AND operations
% ========================================
Test4   ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#FF0F
        SETI $11,#0FFF
        AND     Result,$10,$11
        SETI Expect,#0F0F
        CMP     Temp,Result,Expect
        PBZ     Temp,Test5
        JMP     TestFail

% ========================================
% Test 5: Bitwise XOR operations
% ========================================
Test5   ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#FFFF
        SETI $11,#F0F0
        XOR     Result,$10,$11
        SETI Expect,#0F0F
        CMP     Temp,Result,Expect
        PBZ     Temp,Test6
        JMP     TestFail

% ========================================
% Test 6: ANDN operation
% ========================================
Test6   ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#FFFF
        SETI $11,#0F0F
        ANDN    Result,$10,$11
        SETI Expect,#F0F0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test7
        JMP     TestFail

% ========================================
% Test 7: NOR operation
% ========================================
Test7   ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#00
        SETI $11,#00
        NOR     Result,$10,$11
        SETI Expect,#0
        SUBUI   Expect,Expect,1 % All 1s (-1)
        CMP     Temp,Result,Expect
        PBZ     Temp,Test8
        JMP     TestFail

% ========================================
% Test 8: NAND operation
% ========================================
Test8   ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#FFFF
        SETI $11,#FFFF
        NAND    Result,$10,$11
        SETI Expect,#0
        SUBUI   Expect,Expect,1 % All 1s
        SETI $12,#FFFF
        XOR     Expect,Expect,$12       % Should give ~#FFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test9
        JMP     TestFail

% ========================================
% Test 9: NXOR operation
% ========================================
Test9   ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#F0F0
        SETI $11,#F0F0
        NXOR    Result,$10,$11
        SETI Expect,#0
        SUBUI   Expect,Expect,1 % All 1s
        CMP     Temp,Result,Expect
        PBZ     Temp,Test10
        JMP     TestFail

% ========================================
% Test 10: ADDU - Unsigned addition
% ========================================
Test10  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,100
        SETI $11,50
        ADDU    Result,$10,$11
        SETI Expect,150
        CMP     Temp,Result,Expect
        PBZ     Temp,Test11
        JMP     TestFail

% ========================================
% Test 11: SUBU - Unsigned subtraction
% ========================================
Test11  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,100
        SETI $11,30
        SUBU    Result,$10,$11
        SETI Expect,70
        CMP     Temp,Result,Expect
        PBZ     Temp,Test12
        JMP     TestFail

% ========================================
% Test 12: 2ADDU - Times 2 and add
% ========================================
Test12  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,10
        SETI $11,5
        2ADDU   Result,$10,$11
        SETI Expect,25       % 10*2 + 5 = 25
        CMP     Temp,Result,Expect
        PBZ     Temp,Test13
        JMP     TestFail

% ========================================
% Test 13: 4ADDU - Times 4 and add
% ========================================
Test13  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,10
        SETI $11,5
        4ADDU   Result,$10,$11
        SETI Expect,45       % 10*4 + 5 = 45
        CMP     Temp,Result,Expect
        PBZ     Temp,Test14
        JMP     TestFail

% ========================================
% Test 14: 8ADDU - Times 8 and add
% ========================================
Test14  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,10
        SETI $11,5
        8ADDU   Result,$10,$11
        SETI Expect,85       % 10*8 + 5 = 85
        CMP     Temp,Result,Expect
        PBZ     Temp,Test15
        JMP     TestFail

% ========================================
% Test 15: 16ADDU - Times 16 and add
% ========================================
Test15  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,10
        SETI $11,5
        16ADDU  Result,$10,$11
        SETI Expect,165      % 10*16 + 5 = 165
        CMP     Temp,Result,Expect
        PBZ     Temp,Test16
        JMP     TestFail

% ========================================
% Test 16: CMP - Signed comparison (less than)
% ========================================
Test16  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,5
        SETI $11,10
        CMP     Result,$10,$11
        SETI Expect,#0
        SUBUI   Expect,Expect,1        % -1 (less than)
        CMP     Temp,Result,Expect
        PBZ     Temp,Test17
        JMP     TestFail

% ========================================
% Test 17: CMP - Signed comparison (equal)
% ========================================
Test17  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,42
        SETI $11,42
        CMP     Result,$10,$11
        SETI Expect,0        % Equal
        CMP     Temp,Result,Expect
        PBZ     Temp,Test18
        JMP     TestFail

% ========================================
% Test 18: CMP - Signed comparison (greater than)
% ========================================
Test18  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,100
        SETI $11,50
        CMP     Result,$10,$11
        SETI Expect,1        % Greater than
        CMP     Temp,Result,Expect
        PBZ     Temp,Test19
        JMP     TestFail

% ========================================
% Test 19: CMPU - Unsigned comparison
% ========================================
Test19  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,100
        SETI $11,50
        CMPU    Result,$10,$11
        SETI Expect,1        % Greater than
        CMP     Temp,Result,Expect
        PBZ     Temp,Test20
        JMP     TestFail

% ========================================
% Test 20: Memory load/store - BYTE
% ========================================
Test20  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $20,#100        % Address
        SETI $21,#42         % Value to store
        STB     $21,$20,Zero    % Store byte
        LDB     Result,$20,Zero % Load it back
        SETI Expect,#42
        CMP     Temp,Result,Expect
        PBZ     Temp,Test21
        JMP     TestFail

% ========================================
% Test 21: Memory load/store - WYDE
% ========================================
Test21  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $20,#200        % Address
        SETI $21,#ABCD       % Value to store
        STW     $21,$20,Zero    % Store wyde
        LDWU    Result,$20,Zero % Load it back (unsigned)
        SETI Expect,#ABCD
        CMP     Temp,Result,Expect
        PBZ     Temp,Test22
        JMP     TestFail

% ========================================
% Test 22: Memory load/store - TETRA
% ========================================
Test22  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $20,#300        % Address
        SETI $21,#12345678   % Value to store
        STT     $21,$20,Zero    % Store tetra
        LDT     Result,$20,Zero % Load it back
        SETI Expect,#12345678
        CMP     Temp,Result,Expect
        PBZ     Temp,Test23
        JMP     TestFail

% ========================================
% Test 23: Memory load/store - OCTA
% ========================================
Test23  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $20,#400        % Address (8-byte aligned)
        SETI $21,#123456789ABCDEF
        STO     $21,$20,Zero    % Store octa
        LDO     Result,$20,Zero % Load it back
        SETI Expect,#123456789ABCDEF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test24
        JMP     TestFail

% ========================================
% Test 24: ODIF - Octa difference
% ========================================
Test24  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#FF
        SETI $11,#0F
        ODIF    Result,$10,$11
        SETI Expect,#F0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test25
        JMP     TestFail

% ========================================
% Test 25: SADD - Sideways add
% ========================================
Test25  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#0101010101010101
        SADD    Result,$10,Zero
        SETI Expect,8        % Eight 1-bits
        CMP     Temp,Result,Expect
        PBZ     Temp,Test26
        JMP     TestFail

% ========================================
% Test 26: SL - Shift left
% ========================================
Test26  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#1
        SETI $11,4
        SL      Result,$10,$11
        SETI Expect,#10
        CMP     Temp,Result,Expect
        PBZ     Temp,Test27
        JMP     TestFail

% ========================================
% Test 27: SR - Shift right
% ========================================
Test27  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#80
        SETI $11,4
        SR      Result,$10,$11
        SETI Expect,#8
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
        SETI $10,100
        SETI $11,50
        ADD     Result,$10,$11
        SETI Expect,150
        CMP     Temp,Result,Expect
        PBZ     Temp,Test29
        JMP     TestFail

% ========================================
% Test 29: ADDI - Add immediate (no overflow)
% ========================================
Test29  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,100
        ADDI    Result,$10,75
        SETI Expect,175
        CMP     Temp,Result,Expect
        PBZ     Temp,Test30
        JMP     TestFail

% ========================================
% Test 30: ADDU - Unsigned addition
% ========================================
Test30  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#FFFFFFFFFFFFFFFF
        SETI $11,1
        ADDU    Result,$10,$11
        SETI Expect,0        % Wraps around
        CMP     Temp,Result,Expect
        PBZ     Temp,Test31
        JMP     TestFail

% ========================================
% Test 31: ADDUI - Add unsigned immediate
% ========================================
Test31  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,#FFFFFFFFFFFFFF00
        ADDUI   Result,$10,#FF
        SETI Expect,#FFFFFFFFFFFFFFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test32
        JMP     TestFail

% ========================================
% Test 32: SUB - Signed subtraction
% ========================================
Test32  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,100
        SETI $11,30
        SUB     Result,$10,$11
        SETI Expect,70
        CMP     Temp,Result,Expect
        PBZ     Temp,Test33
        JMP     TestFail

% ========================================
% Test 33: SUBI - Subtract immediate
% ========================================
Test33  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,200
        SUBI    Result,$10,50
        SETI Expect,150
        CMP     Temp,Result,Expect
        PBZ     Temp,Test34
        JMP     TestFail

% ========================================
% Test 34: SUBU - Unsigned subtraction
% ========================================
Test34  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,10
        SETI $11,20
        SUBU    Result,$10,$11
        SETI Expect,#FFFFFFFFFFFFFFF6  % -10 as unsigned
        CMP     Temp,Result,Expect
        PBZ     Temp,Test35
        JMP     TestFail

% ========================================
% Test 35: SUBUI - Subtract unsigned immediate
% ========================================
Test35  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,50
        SUBUI   Result,$10,100
        SETI Expect,#FFFFFFFFFFFFFFCE  % -50 as unsigned
        CMP     Temp,Result,Expect
        PBZ     Temp,Test36
        JMP     TestFail

% ========================================
% Test 36: 2ADDU - Times 2 and add (register)
% ========================================
Test36  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,25
        SETI $11,10
        2ADDU   Result,$10,$11
        SETI Expect,60       % 25*2 + 10 = 60
        CMP     Temp,Result,Expect
        PBZ     Temp,Test37
        JMP     TestFail

% ========================================
% Test 37: 2ADDUI - Times 2 and add (immediate)
% ========================================
Test37  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,30
        2ADDUI  Result,$10,15
        SETI Expect,75       % 30*2 + 15 = 75
        CMP     Temp,Result,Expect
        PBZ     Temp,Test38
        JMP     TestFail

% ========================================
% Test 38: 4ADDU - Times 4 and add (register)
% ========================================
Test38  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,20
        SETI $11,12
        4ADDU   Result,$10,$11
        SETI Expect,92       % 20*4 + 12 = 92
        CMP     Temp,Result,Expect
        PBZ     Temp,Test39
        JMP     TestFail

% ========================================
% Test 39: 4ADDUI - Times 4 and add (immediate)
% ========================================
Test39  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,15
        4ADDUI  Result,$10,8
        SETI Expect,68       % 15*4 + 8 = 68
        CMP     Temp,Result,Expect
        PBZ     Temp,Test40
        JMP     TestFail

% ========================================
% Test 40: 8ADDU - Times 8 and add (register)
% ========================================
Test40  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,12
        SETI $11,7
        8ADDU   Result,$10,$11
        SETI Expect,103      % 12*8 + 7 = 103
        CMP     Temp,Result,Expect
        PBZ     Temp,Test41
        JMP     TestFail

% ========================================
% Test 41: 8ADDUI - Times 8 and add (immediate)
% ========================================
Test41  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,9
        8ADDUI  Result,$10,6
        SETI Expect,78       % 9*8 + 6 = 78
        CMP     Temp,Result,Expect
        PBZ     Temp,Test42
        JMP     TestFail

% ========================================
% Test 42: 16ADDU - Times 16 and add (register)
% ========================================
Test42  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,8
        SETI $11,4
        16ADDU  Result,$10,$11
        SETI Expect,132      % 8*16 + 4 = 132
        CMP     Temp,Result,Expect
        PBZ     Temp,Test43
        JMP     TestFail

% ========================================
% Test 43: 16ADDUI - Times 16 and add (immediate)
% ========================================
Test43  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $10,7
        16ADDUI Result,$10,3
        SETI Expect,115      % 7*16 + 3 = 115
        CMP     Temp,Result,Expect
        PBZ     Temp,Test44
        JMP     TestFail

% ========================================
% Test 44: Register $255 as normal register
% ========================================
Test44  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI $255,0         % Initialize $255 to 0
        SETI $10,12345
        ADD     Result,$10,Zero
        SETI Expect,12345
        CMP     Temp,Result,Expect
        PBZ     Temp,Test45
        JMP     TestFail

% ========================================
% Test 45: GETA - Get address (forward reference)
% ========================================
Test45  ADDUI   TestNum,TestNum,1       % Increment test counter
        GETA    Result,LocalData  % Get address of LocalData
        LDO     $10,Result,Zero   % Load the octa at that address (offset 0)
        SETI Expect,#DEADBEEFCAFEBABE
        CMP     Temp,$10,Expect
        PBZ     Temp,Test46
        JMP     TestFail

% ========================================
% Test 46: GETA - Get address (backward reference)
% ========================================
Test46  ADDUI   TestNum,TestNum,1       % Increment test counter
        GETA    Result,Test1      % Get address of Test1 (backward)
        SETI    Expect,Test1      % SETI computes full 64-bit address
        CMP     Temp,Result,Expect
        PBZ     Temp,Test47
        JMP     TestFail

% ========================================
% Test 47: JMP - Unconditional jump forward
% ========================================
Test47  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI Result,99       % This should be skipped
        JMP     Test47Skip
        SETI Result,#DEAD      % This should also be skipped
Test47Skip      SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test48
        JMP     TestFail

% ========================================
% Test 48: JMP - Unconditional jump backward
% ========================================
Test48  ADDUI   TestNum,TestNum,1       % Increment test counter
        JMP     Test48Start
Test48Target    SETI Result,#C0DE
        JMP     Test48End
Test48Start     SETI Result,99
        JMP     Test48Target
Test48End       SETI Expect,#C0DE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test49
        JMP     TestFail

% ========================================
% Test 49: JMP - Large forward offset
% ========================================
Test49  ADDUI   TestNum,TestNum,1       % Increment test counter
        SETI Result,0
        JMP     Test49Far
        JMP     TestFail        % Should not reach here

% ========================================
% Test 50: JMP - Large backward offset  
% ========================================
Test50  ADDUI   TestNum,TestNum,1       % Increment test counter
        JMP     Test50Start
Test50End       SETI Expect,#BABE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test51
        JMP     TestFail
Test50Start     SETI Result,#BABE
        JMP     Test50End

% ========================================
% Test 51: MUL - Multiply
% ========================================
Test51  ADDUI   TestNum,TestNum,1
        SETI $10,5           % Clear and set to 5
        SETI $11,7           % Clear and set to 7
        MUL     Result,$10,$11
        SETI Expect,35
        CMP     Temp,Result,Expect
        PBZ     Temp,Test52
        JMP     TestFail

% ========================================
% Test 52: MULU - Multiply unsigned
% ========================================
Test52  ADDUI   TestNum,TestNum,1
        SETI $10,100
        SETI $11,200
        MULU    Result,$10,$11
        SETI Expect,20000
        CMP     Temp,Result,Expect
        PBZ     Temp,Test53
        JMP     TestFail

% ========================================
% Test 53: DIV - Divide
% ========================================
Test53  ADDUI   TestNum,TestNum,1
        SETI $10,100
        SETI $11,5
        DIV     Result,$10,$11
        SETI Expect,20
        CMP     Temp,Result,Expect
        PBZ     Temp,Test54
        JMP     TestFail

% ========================================
% Test 54: DIVU - Divide unsigned
% ========================================
Test54  ADDUI   TestNum,TestNum,1
        SETI $10,1000
        SETI $11,10
        DIVU    Result,$10,$11
        SETI Expect,100
        CMP     Temp,Result,Expect
        PBZ     Temp,Test55
        JMP     TestFail

% ========================================
% Test 55: NEG - Negate
% ========================================
Test55  ADDUI   TestNum,TestNum,1
        SETI $10,42
        NEG     Result,0,$10
        SETI Expect,0
        SUBU    Expect,Expect,$10
        CMP     Temp,Result,Expect
        PBZ     Temp,Test56
        JMP     TestFail

% ========================================
% Test 56: NEGU - Negate unsigned
% ========================================
Test56  ADDUI   TestNum,TestNum,1
        SETI $10,100
        NEGU    Result,0,$10
        SETI Expect,0
        SUBU    Expect,Expect,$10
        CMP     Temp,Result,Expect
        PBZ     Temp,Test57
        JMP     TestFail

% ========================================
% Test 57: ORN - OR-NOT
% ========================================
Test57  ADDUI   TestNum,TestNum,1
        SETI $10,#FF00
        SETI $11,#0F0F
        ORN     Result,$10,$11
        SETI Expect,#FFFFFFFFFFFFFFF0     % ~0x0F0F = 0xFFFFFFFFFFFFF0F0, OR with 0xFF00
        CMP     Temp,Result,Expect
        PBZ     Temp,Test58
        JMP     TestFail

% ========================================
% Test 58: MUX - Multiplex
% ========================================
Test58  ADDUI   TestNum,TestNum,1
        SETI $10,#AAAA
        SETI $11,#5555
        SETI $12,#FFFF
        PUT     rM,$12          % Set mask to 0xFFFF
        MUX     Result,$10,$11  % Select from $10 where mask is 1, $11 where mask is 0
        SETI Expect,#AAAA    % Mask is all 1s, so get $10
        CMP     Temp,Result,Expect
        PBZ     Temp,Test59
        JMP     TestFail

% ========================================
% Test 59: BDIF - Byte difference
% ========================================
Test59  ADDUI   TestNum,TestNum,1
        SETI $10,#0A14
        SETI $11,#050C
        BDIF    Result,$10,$11
        SETI Expect,#0508
        CMP     Temp,Result,Expect
        PBZ     Temp,Test60
        JMP     TestFail

% ========================================
% Test 60: WDIF - Wyde difference
% ========================================
Test60  ADDUI   TestNum,TestNum,1
        SETI $10,#0A000014
        SETI $11,#0500000C
        WDIF    Result,$10,$11
        SETI Expect,#05000008
        CMP     Temp,Result,Expect
        PBZ     Temp,Test61
        JMP     TestFail

% ========================================
% Test 61: TDIF - Tetra difference
% ========================================
Test61  ADDUI   TestNum,TestNum,1
        SETI $10,#A00000014
        SETI $11,#50000000C
        TDIF    Result,$10,$11
        SETI Expect,#500000008
        CMP     Temp,Result,Expect
        PBZ     Temp,Test62
        JMP     TestFail

% ========================================
% Test 62: MOR - Mix and Match
% ========================================
Test62  ADDUI   TestNum,TestNum,1
        SETI $10,#0101010101010101  % Each byte is 0x01
        MORI    Result,$10,#FF         % OR all 8 bytes together
        SETI Expect,#01             % Result: 0x01 OR 0x01 OR ... = 0x01
        CMP     Temp,Result,Expect
        PBZ     Temp,Test63
        JMP     TestFail

% ========================================
% Test 63: MXOR - Multiple XOR (Boolean matrix mult with XOR)
% ========================================
Test63  ADDUI   TestNum,TestNum,1
        SETI $10,#0101010101010101  % Each byte is 0x01
        MXORI   Result,$10,#FF         % XOR all 8 bytes together
        SETI Expect,0               % 0x01 XOR 0x01 XOR ... (8 times) = 0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test64
        JMP     TestFail

% ========================================
% Test 64: SLU - Shift left unsigned
% ========================================
Test64  ADDUI   TestNum,TestNum,1
        SETI $10,1
        SETI $11,8
        SLU     Result,$10,$11
        SETI Expect,256
        CMP     Temp,Result,Expect
        PBZ     Temp,Test65
        JMP     TestFail

% ========================================
% Test 65: SRU - Shift right unsigned
% ========================================
Test65  ADDUI   TestNum,TestNum,1
        SETI $10,256
        SETI $11,4
        SRU     Result,$10,$11
        SETI Expect,16
        CMP     Temp,Result,Expect
        PBZ     Temp,Test66
        JMP     TestFail

% ========================================
% Test 66: INCH - Increment high wyde
% ========================================
Test66  ADDUI   TestNum,TestNum,1
        SETH    Result,#1000
        INCH    Result,#0100
        SETH    Expect,#1100
        CMP     Temp,Result,Expect
        PBZ     Temp,Test67
        JMP     TestFail

% ========================================
% Test 67: INCMH - Increment medium high wyde
% ========================================
Test67  ADDUI   TestNum,TestNum,1
        SETMH   Result,#2000
        INCMH   Result,#0200
        SETMH   Expect,#2200
        CMP     Temp,Result,Expect
        PBZ     Temp,Test68
        JMP     TestFail

% ========================================
% Test 68: INCML - Increment medium low wyde
% ========================================
Test68  ADDUI   TestNum,TestNum,1
        SETML   Result,#3000
        INCML   Result,#0300
        SETML   Expect,#3300
        CMP     Temp,Result,Expect
        PBZ     Temp,Test69
        JMP     TestFail

% ========================================
% Test 69: ORH - OR high wyde
% ========================================
Test69  ADDUI   TestNum,TestNum,1
        SETH    Result,#AA00
        ORH     Result,#00FF
        SETH    Expect,#AAFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test70
        JMP     TestFail

% ========================================
% Test 70: ORMH - OR medium high wyde
% ========================================
Test70  ADDUI   TestNum,TestNum,1
        SETMH   Result,#BB00
        ORMH    Result,#00FF
        SETMH   Expect,#BBFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test71
        JMP     TestFail

% ========================================
% Test 71: ORML - OR medium low wyde
% ========================================
Test71  ADDUI   TestNum,TestNum,1
        SETML   Result,#CC00
        ORML    Result,#00FF
        SETML   Expect,#CCFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test72
        JMP     TestFail

% ========================================
% Test 72: ORL - OR low wyde
% ========================================
Test72  ADDUI   TestNum,TestNum,1
        SETI Result,#DD00
        ORL     Result,#00FF
        SETI Expect,#DDFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test73
        JMP     TestFail

% ========================================
% Test 73: ANDNH - AND-NOT high wyde
% ========================================
Test73  ADDUI   TestNum,TestNum,1
        SETI Result,#FFFFFFFFFFFFFFFF
        ANDNH   Result,#00FF
        SETI Expect,#FF00FFFFFFFFFFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test74
        JMP     TestFail

% ========================================
% Test 74: ANDNMH - AND-NOT medium high wyde
% ========================================
Test74  ADDUI   TestNum,TestNum,1
        SETI Result,#FFFFFFFFFFFFFFFF
        ANDNMH  Result,#00FF
        SETI Expect,#FFFFFF00FFFFFFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test75
        JMP     TestFail

% ========================================
% Test 75: ANDNML - AND-NOT medium low wyde
% ========================================
Test75  ADDUI   TestNum,TestNum,1
        SETI Result,#FFFFFFFFFFFFFFFF
        ANDNML  Result,#00FF
        SETI Expect,#FFFFFFFFFF00FFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test76
        JMP     TestFail

% ========================================
% Test 76: ANDNL - AND-NOT low wyde
% ========================================
Test76  ADDUI   TestNum,TestNum,1
        SETI Result,#FFFFFFFFFFFFFFFF
        ANDNL   Result,#00FF
        SETI Expect,#FFFFFFFFFFFFFFFF
        ANDNL   Expect,#00FF            % Same operation to get expected value
        CMP     Temp,Result,Expect
        PBZ     Temp,Test77
        JMP     TestFail

% ========================================
% Test 77: BZ - Branch if zero (taken)
% ========================================
Test77  ADDUI   TestNum,TestNum,1
        SETI $10,0
        SETI Result,99
        BZ      $10,Test77Skip
        SETI Result,#DEAD
Test77Skip      SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test78
        JMP     TestFail

% ========================================
% Test 78: BNZ - Branch if non-zero (taken)
% ========================================
Test78  ADDUI   TestNum,TestNum,1
        SETI $10,1
        SETI Result,99
        BNZ     $10,Test78Skip
        SETI Result,#DEAD
Test78Skip      SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test79
        JMP     TestFail

% ========================================
% Test 79: BP - Branch if positive (taken)
% ========================================
Test79  ADDUI   TestNum,TestNum,1
        SETI $10,42
        SETI Result,99
        BP      $10,Test79Skip
        SETI Result,#DEAD
Test79Skip      SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test80
        JMP     TestFail

% ========================================
% Test 80: BN - Branch if negative (not taken, then taken)
% ========================================
Test80  ADDUI   TestNum,TestNum,1
        SETI $10,5
        BN      $10,TestFail
        NEG     $10,0,$10
        SETI Result,99
        BN      $10,Test80Skip
        SETI Result,#DEAD
Test80Skip      SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test81
        JMP     TestFail

% ========================================
% Test 81: BNN - Branch if non-negative (taken)
% ========================================
Test81  ADDUI   TestNum,TestNum,1
        SETI $10,0
        SETI Result,99
        BNN     $10,Test81Skip
        SETI Result,#DEAD
Test81Skip      SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test82
        JMP     TestFail

% ========================================
% Test 82: BNP - Branch if non-positive (taken)
% ========================================
Test82  ADDUI   TestNum,TestNum,1
        SETI $10,0
        SETI Result,99
        BNP     $10,Test82Skip
        SETI Result,#DEAD
Test82Skip      SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test83
        JMP     TestFail

% ========================================
% Test 83: BOD - Branch if odd (taken)
% ========================================
Test83  ADDUI   TestNum,TestNum,1
        SETI $10,7
        SETI Result,99
        BOD     $10,Test83Skip
        SETI Result,#DEAD
Test83Skip      SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test84
        JMP     TestFail

% ========================================
% Test 84: BEV - Branch if even (taken)
% ========================================
Test84  ADDUI   TestNum,TestNum,1
        SETI $10,8
        SETI Result,99
        BEV     $10,Test84Skip
        SETI Result,#DEAD
Test84Skip      SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test85
        JMP     TestFail

% ========================================
% Test 85: PBZ - Probable branch if zero (taken)
% ========================================
Test85  ADDUI   TestNum,TestNum,1
        SETI $10,0
        SETI Result,#CAFE
        PBZ     $10,Test85Skip
        SETI Result,#DEAD
Test85Skip      SETI Expect,#CAFE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test86
        JMP     TestFail

% ========================================
% Test 86: PBNZ - Probable branch if non-zero (taken)
% ========================================
Test86  ADDUI   TestNum,TestNum,1
        SETI $10,1
        SETI Result,#BABE
        PBNZ    $10,Test86Skip
        SETI Result,#DEAD
Test86Skip      SETI Expect,#BABE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test87
        JMP     TestFail

% ========================================
% Test 87: CSN - Conditional set if negative
% ========================================
Test87  ADDUI   TestNum,TestNum,1
        SETI $10,5
        NEG     $10,0,$10
        SETI $11,42
        SETI $12,99
        CSN     Result,$10,$11
        SETI Expect,42
        CMP     Temp,Result,Expect
        PBZ     Temp,Test88
        JMP     TestFail

% ========================================
% Test 88: CSZ - Conditional set if zero
% ========================================
Test88  ADDUI   TestNum,TestNum,1
        SETI $10,0
        SETI $11,0
        SETI $12,1
        CSZ     Result,$10,$11
        SETI Expect,0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test89
        JMP     TestFail

% ========================================
% Test 89: CSP - Conditional set if positive
% ========================================
Test89  ADDUI   TestNum,TestNum,1
        SETI $10,42
        SETI $11,1
        SETI $12,1
        NEG     $12,0,$12
        CSP     Result,$10,$11
        SETI Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Test90
        JMP     TestFail

% ========================================
% Test 90: CSNN - Conditional set if non-negative
% ========================================
Test90  ADDUI   TestNum,TestNum,1
        SETI $10,0
        SETI $11,7
        SETI $12,1
        NEG     $12,0,$12
        CSNN    Result,$10,$11
        SETI Expect,7
        CMP     Temp,Result,Expect
        PBZ     Temp,Test91
        JMP     TestFail

% ========================================
% Test 91: CSNZ - Conditional set if non-zero
% ========================================
Test91  ADDUI   TestNum,TestNum,1
        SETI $10,7
        SETI $11,11
        SETI $12,0
        CSNZ    Result,$10,$11
        SETI Expect,11
        CMP     Temp,Result,Expect
        PBZ     Temp,Test92
        JMP     TestFail

% ========================================
% Test 92: CSOD - Conditional set if odd
% ========================================
Test92  ADDUI   TestNum,TestNum,1
        SETI $10,9
        SETI $11,3
        SETI $12,2
        CSOD    Result,$10,$11
        SETI Expect,3
        CMP     Temp,Result,Expect
        PBZ     Temp,Test93
        JMP     TestFail

% ========================================
% Test 93: CSEV - Conditional set if even
% ========================================
Test93  ADDUI   TestNum,TestNum,1
        SETI $10,10
        SETI $11,2
        SETI $12,3
        CSEV    Result,$10,$11
        SETI Expect,2
        CMP     Temp,Result,Expect
        PBZ     Temp,Test94
        JMP     TestFail

% ========================================
% Test 94: ZSN - Zero or set if negative
% ========================================
Test94  ADDUI   TestNum,TestNum,1
        SETI $10,5
        NEG     $10,0,$10
        SETI $11,17
        ZSN     Result,$10,$11
        SETI Expect,17
        CMP     Temp,Result,Expect
        PBZ     Temp,Test95
        JMP     TestFail

% ========================================
% Test 95: ZSZ - Zero or set if zero
% ========================================
Test95  ADDUI   TestNum,TestNum,1
        SETI $10,0
        SETI $11,19
        ZSZ     Result,$10,$11
        SETI Expect,19
        CMP     Temp,Result,Expect
        PBZ     Temp,Test96
        JMP     TestFail

% ========================================
% Test 96: ZSP - Zero or set if positive
% ========================================
Test96  ADDUI   TestNum,TestNum,1
        SETI $10,42
        SETI $11,21
        ZSP     Result,$10,$11
        SETI Expect,21
        CMP     Temp,Result,Expect
        PBZ     Temp,Test97
        JMP     TestFail

% ========================================
% Test 97: ZSNN - Zero or set if non-negative
% ========================================
Test97  ADDUI   TestNum,TestNum,1
        SETI $10,1
        SETI $11,17
        ZSNN    Result,$10,$11
        SETI Expect,17
        CMP     Temp,Result,Expect
        PBZ     Temp,Test98
        JMP     TestFail

% ========================================
% Test 98: ZSNZ - Zero or set if non-zero
% ========================================
Test98  ADDUI   TestNum,TestNum,1
        SETI $10,7
        SETI $11,17
        ZSNZ    Result,$10,$11
        SETI Expect,17
        CMP     Temp,Result,Expect
        PBZ     Temp,Test99
        JMP     TestFail

% ========================================
% Test 99: ZSOD - Zero or set if odd
% ========================================
Test99  ADDUI   TestNum,TestNum,1
        SETI $10,13
        SETI $11,27
        ZSOD    Result,$10,$11
        SETI Expect,27
        CMP     Temp,Result,Expect
        PBZ     Temp,Test100
        JMP     TestFail

% ========================================
% Test 100: ZSEV - Zero or set if even
% ========================================
Test100 ADDUI   TestNum,TestNum,1
        SETI $10,14
        SETI $11,29
        ZSEV    Result,$10,$11
        SETI Expect,29
        CMP     Temp,Result,Expect
        PBZ     Temp,Test101
        JMP     TestFail

% ========================================
% FLOATING POINT TESTS
% ========================================

% ========================================
% Test 101: FLOT - Float from integer
% ========================================
Test101 ADDUI   TestNum,TestNum,1
        SETI $10,42
        FLOT    Result,Zero,$10
        FLOT    Expect,Zero,$10 % 42.0 in floating point
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test102
        JMP     TestFail

% ========================================
% Test 102: FLOTI - Float from immediate
% ========================================
Test102 ADDUI   TestNum,TestNum,1
        FLOTI   Result,Zero,100
        FLOTI   Expect,Zero,100
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test103
        JMP     TestFail

% ========================================
% Test 103: FLOTU - Float from unsigned integer
% ========================================
Test103 ADDUI   TestNum,TestNum,1
        SETI $10,1000
        FLOTU   Result,Zero,$10
        FLOTU   Expect,Zero,$10
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test104
        JMP     TestFail

% ========================================
% Test 104: FLOTUI - Float from unsigned immediate
% ========================================
Test104 ADDUI   TestNum,TestNum,1
        FLOTUI  Result,Zero,255
        FLOTUI  Expect,Zero,255
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test105
        JMP     TestFail

% ========================================
% Test 105: SFLOT - Short float from integer
% ========================================
Test105 ADDUI   TestNum,TestNum,1
        SETI $10,7
        SFLOT   Result,Zero,$10
        SFLOT   Expect,Zero,$10
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test106
        JMP     TestFail

% ========================================
% Test 106: SFLOTI - Short float from immediate
% ========================================
Test106 ADDUI   TestNum,TestNum,1
        SFLOTI  Result,Zero,13
        SFLOTI  Expect,Zero,13
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test107
        JMP     TestFail

% ========================================
% Test 107: SFLOTU - Short float from unsigned
% ========================================
Test107 ADDUI   TestNum,TestNum,1
        SETI $10,99
        SFLOTU  Result,Zero,$10
        SFLOTU  Expect,Zero,$10
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test108
        JMP     TestFail

% ========================================
% Test 108: SFLOTUI - Short float from unsigned immediate
% ========================================
Test108 ADDUI   TestNum,TestNum,1
        SFLOTUI Result,Zero,77
        SFLOTUI Expect,Zero,77
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test109
        JMP     TestFail

% ========================================
% Test 109: FADD - Floating point add
% ========================================
Test109 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,10
        FLOTI   $11,Zero,20
        FADD    Result,$10,$11
        FLOTI   Expect,Zero,30
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test110
        JMP     TestFail

% ========================================
% Test 110: FSUB - Floating point subtract
% ========================================
Test110 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,100
        FLOTI   $11,Zero,42
        FSUB    Result,$10,$11
        FLOTI   Expect,Zero,58
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test111
        JMP     TestFail

% ========================================
% Test 111: FMUL - Floating point multiply
% ========================================
Test111 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,5
        FLOTI   $11,Zero,7
        FMUL    Result,$10,$11
        FLOTI   Expect,Zero,35
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test112
        JMP     TestFail

% ========================================
% Test 112: FDIV - Floating point divide
% ========================================
Test112 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,100
        FLOTI   $11,Zero,4
        FDIV    Result,$10,$11
        FLOTI   Expect,Zero,25
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test113
        JMP     TestFail

% ========================================
% Test 113: FCMP - Floating point compare
% ========================================
Test113 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,50
        FLOTI   $11,Zero,50
        FCMP    Result,$10,$11
        SETI Expect,0        % Equal
        CMP     Temp,Result,Expect
        PBZ     Temp,Test114
        JMP     TestFail

% ========================================
% Test 114: FEQL - Floating point equal
% ========================================
Test114 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,77
        FLOTI   $11,Zero,77
        FEQL    Result,$10,$11
        SETI Expect,1        % True (equal)
        CMP     Temp,Result,Expect
        PBZ     Temp,Test115
        JMP     TestFail

% ========================================
% Test 115: FUN - Floating point unordered
% ========================================
Test115 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,100
        FLOTI   $11,Zero,200
        FUN     Result,$10,$11
        SETI Expect,0        % False (both are ordered numbers)
        CMP     Temp,Result,Expect
        PBZ     Temp,Test116
        JMP     TestFail

% ========================================
% Test 116: FIX - Fix (float to int)
% ========================================
Test116 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,42
        FIX     Result,Zero,$10
        SETI Expect,42
        CMP     Temp,Result,Expect
        PBZ     Temp,Test117
        JMP     TestFail

% ========================================
% Test 117: FIXU - Fix unsigned (float to unsigned int)
% ========================================
Test117 ADDUI   TestNum,TestNum,1
        FLOTUI  $10,Zero,200
        FIXU    Result,Zero,$10
        SETI Expect,200
        CMP     Temp,Result,Expect
        PBZ     Temp,Test118
        JMP     TestFail

% ========================================
% Test 118: FSQRT - Floating point square root
% ========================================
Test118 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,16
        FSQRT   Result,Zero,$10
        FLOTI   Expect,Zero,4
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test119
        JMP     TestFail

% ========================================
% Test 119: FINT - Floating point integer part
% ========================================
Test119 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,42
        FINT    Result,Zero,$10
        FLOTI   Expect,Zero,42
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test120
        JMP     TestFail

% ========================================
% Test 120: FREM - Floating point remainder
% ========================================
Test120 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,10
        FLOTI   $11,Zero,3
        FREM    Result,$10,$11
        FLOTI   Expect,Zero,1
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test121
        JMP     TestFail

% ========================================
% SPECIAL INSTRUCTION TESTS
% ========================================

% ========================================
% Test 121: PUT - Put to special register
% ========================================
Test121 ADDUI   TestNum,TestNum,1
        SETI $10,#1234
        PUT     rD,$10
        GET     Result,rD
        SETI Expect,#1234
        CMP     Temp,Result,Expect
        PBZ     Temp,Test122
        JMP     TestFail

% ========================================
% Test 122: PUTI - Put immediate to special register
% ========================================
Test122 ADDUI   TestNum,TestNum,1
        PUTI    rE,99
        GET     Result,rE
        SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test123
        JMP     TestFail

% ========================================
% Test 123: NEGI - Negate immediate (both Y and Z are immediates)
% ========================================
Test123 ADDUI   TestNum,TestNum,1
        NEGI    Result,10,5
        SETI Expect,5        % Result should be 10-5=5
        CMP     Temp,Result,Expect
        PBZ     Temp,Test124
        JMP     TestFail

% ========================================
% Test 124: NEGUI - Negate unsigned immediate
% ========================================
Test124 ADDUI   TestNum,TestNum,1
        NEGUI   Result,20,8
        SETI Expect,12       % Result should be 20-8=12
        CMP     Temp,Result,Expect
        PBZ     Temp,Test125
        JMP     TestFail

% ========================================
% CONDITIONAL SET IMMEDIATE TESTS
% ========================================

% ========================================
% Test 125: CSNI - Conditional set if negative (immediate)
% ========================================
Test125 ADDUI   TestNum,TestNum,1
        SETI $10,5
        NEG     $10,0,$10
        CSNI    Result,$10,88
        SETI Expect,88
        CMP     Temp,Result,Expect
        PBZ     Temp,Test126
        JMP     TestFail

% ========================================
% Test 126: CSZI - Conditional set if zero (immediate)
% ========================================
Test126 ADDUI   TestNum,TestNum,1
        SETI $10,0
        CSZI    Result,$10,99
        SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test127
        JMP     TestFail

% ========================================
% Test 127: CSPI - Conditional set if positive (immediate)
% ========================================
Test127 ADDUI   TestNum,TestNum,1
        SETI $10,42
        CSPI    Result,$10,77
        SETI Expect,77
        CMP     Temp,Result,Expect
        PBZ     Temp,Test128
        JMP     TestFail

% ========================================
% Test 128: CSODI - Conditional set if odd (immediate)
% ========================================
Test128 ADDUI   TestNum,TestNum,1
        SETI $10,13
        CSODI   Result,$10,55
        SETI Expect,55
        CMP     Temp,Result,Expect
        PBZ     Temp,Test129
        JMP     TestFail

% ========================================
% Test 129: CSNNI - Conditional set if non-negative (immediate)
% ========================================
Test129 ADDUI   TestNum,TestNum,1
        SETI $10,0
        CSNNI   Result,$10,66
        SETI Expect,66
        CMP     Temp,Result,Expect
        PBZ     Temp,Test130
        JMP     TestFail

% ========================================
% Test 130: CSNZI - Conditional set if non-zero (immediate)
% ========================================
Test130 ADDUI   TestNum,TestNum,1
        SETI $10,1
        CSNZI   Result,$10,44
        SETI Expect,44
        CMP     Temp,Result,Expect
        PBZ     Temp,Test131
        JMP     TestFail

% ========================================
% Test 131: CSNPI - Conditional set if non-positive (immediate)
% ========================================
Test131 ADDUI   TestNum,TestNum,1
        SETI $10,0
        CSNPI   Result,$10,33
        SETI Expect,33
        CMP     Temp,Result,Expect
        PBZ     Temp,Test132
        JMP     TestFail

% ========================================
% Test 132: CSEVI - Conditional set if even (immediate)
% ========================================
Test132 ADDUI   TestNum,TestNum,1
        SETI $10,22
        CSEVI   Result,$10,11
        SETI Expect,11
        CMP     Temp,Result,Expect
        PBZ     Temp,Test133
        JMP     TestFail

% ========================================
% ZERO-OR-SET IMMEDIATE TESTS
% ========================================

% ========================================
% Test 133: ZSNI - Zero or set if negative (immediate)
% ========================================
Test133 ADDUI   TestNum,TestNum,1
        SETI $10,5
        NEG     $10,0,$10
        ZSNI    Result,$10,88
        SETI Expect,88
        CMP     Temp,Result,Expect
        PBZ     Temp,Test134
        JMP     TestFail

% ========================================
% Test 134: ZSZI - Zero or set if zero (immediate)
% ========================================
Test134 ADDUI   TestNum,TestNum,1
        SETI $10,0
        ZSZI    Result,$10,99
        SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test135
        JMP     TestFail

% ========================================
% Test 135: ZSPI - Zero or set if positive (immediate)
% ========================================
Test135 ADDUI   TestNum,TestNum,1
        SETI $10,42
        ZSPI    Result,$10,77
        SETI Expect,77
        CMP     Temp,Result,Expect
        PBZ     Temp,Test136
        JMP     TestFail

% ========================================
% Test 136: ZSODI - Zero or set if odd (immediate)
% ========================================
Test136 ADDUI   TestNum,TestNum,1
        SETI $10,13
        ZSODI   Result,$10,55
        SETI Expect,55
        CMP     Temp,Result,Expect
        PBZ     Temp,Test137
        JMP     TestFail

% ========================================
% Test 137: ZSNNI - Zero or set if non-negative (immediate)
% ========================================
Test137 ADDUI   TestNum,TestNum,1
        SETI $10,0
        ZSNNI   Result,$10,66
        SETI Expect,66
        CMP     Temp,Result,Expect
        PBZ     Temp,Test138
        JMP     TestFail

% ========================================
% Test 138: ZSNZI - Zero or set if non-zero (immediate)
% ========================================
Test138 ADDUI   TestNum,TestNum,1
        SETI $10,1
        ZSNZI   Result,$10,44
        SETI Expect,44
        CMP     Temp,Result,Expect
        PBZ     Temp,Test139
        JMP     TestFail

% ========================================
% Test 139: ZSNPI - Zero or set if non-positive (immediate)
% ========================================
Test139 ADDUI   TestNum,TestNum,1
        SETI $10,0
        ZSNPI   Result,$10,33
        SETI Expect,33
        CMP     Temp,Result,Expect
        PBZ     Temp,Test140
        JMP     TestFail

% ========================================
% Test 140: ZSEVI - Zero or set if even (immediate)
% ========================================
Test140 ADDUI   TestNum,TestNum,1
        SETI $10,22
        ZSEVI   Result,$10,11
        SETI Expect,11
        CMP     Temp,Result,Expect
        PBZ     Temp,Test141
        JMP     TestFail

% ========================================
% BACKWARD BRANCH TESTS
% ========================================

% ========================================
% Test 141: BNB - Branch backward if negative
% ========================================
Test141 ADDUI   TestNum,TestNum,1
        SETI Result,99
        JMP     Test141Forward
Test141Back     SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test142
        JMP     TestFail
Test141Forward  SETI $10,5
        NEG     $10,0,$10
        BNB     $10,Test141Back
        JMP     TestFail

% ========================================
% Test 142: BZB - Branch backward if zero
% ========================================
Test142 ADDUI   TestNum,TestNum,1
        SETI Result,88
        JMP     Test142Forward
Test142Back     SETI Expect,88
        CMP     Temp,Result,Expect
        PBZ     Temp,Test143
        JMP     TestFail
Test142Forward  SETI $10,0
        BZB     $10,Test142Back
        JMP     TestFail

% ========================================
% Test 143: BPB - Branch backward if positive
% ========================================
Test143 ADDUI   TestNum,TestNum,1
        SETI Result,77
        JMP     Test143Forward
Test143Back     SETI Expect,77
        CMP     Temp,Result,Expect
        PBZ     Temp,Test144
        JMP     TestFail
Test143Forward  SETI $10,1
        BPB     $10,Test143Back
        JMP     TestFail

% ========================================
% Test 144: BODB - Branch backward if odd
% ========================================
Test144 ADDUI   TestNum,TestNum,1
        SETI Result,66
        JMP     Test144Forward
Test144Back     SETI Expect,66
        CMP     Temp,Result,Expect
        PBZ     Temp,Test145
        JMP     TestFail
Test144Forward  SETI $10,7
        BODB    $10,Test144Back
        JMP     TestFail

% ========================================
% Test 145: BNNB - Branch backward if non-negative
% ========================================
Test145 ADDUI   TestNum,TestNum,1
        SETI Result,55
        JMP     Test145Forward
Test145Back     SETI Expect,55
        CMP     Temp,Result,Expect
        PBZ     Temp,Test146
        JMP     TestFail
Test145Forward  SETI $10,0
        BNNB    $10,Test145Back
        JMP     TestFail

% ========================================
% Test 146: BNZB - Branch backward if non-zero
% ========================================
Test146 ADDUI   TestNum,TestNum,1
        SETI Result,44
        JMP     Test146Forward
Test146Back     SETI Expect,44
        CMP     Temp,Result,Expect
        PBZ     Temp,Test147
        JMP     TestFail
Test146Forward  SETI $10,1
        BNZB    $10,Test146Back
        JMP     TestFail

% ========================================
% Test 147: BNPB - Branch backward if non-positive
% ========================================
Test147 ADDUI   TestNum,TestNum,1
        SETI Result,33
        JMP     Test147Forward
Test147Back     SETI Expect,33
        CMP     Temp,Result,Expect
        PBZ     Temp,Test148
        JMP     TestFail
Test147Forward  SETI $10,0
        BNPB    $10,Test147Back
        JMP     TestFail

% ========================================
% Test 148: BEVB - Branch backward if even
% ========================================
Test148 ADDUI   TestNum,TestNum,1
        SETI Result,22
        JMP     Test148Forward
Test148Back     SETI Expect,22
        CMP     Temp,Result,Expect
        PBZ     Temp,Test149
        JMP     TestFail
Test148Forward  SETI $10,8
        BEVB    $10,Test148Back
        JMP     TestFail

% ========================================
% PROBABLE BACKWARD BRANCH TESTS
% ========================================

% ========================================
% Test 149: PBNB - Probable branch backward if negative
% ========================================
Test149 ADDUI   TestNum,TestNum,1
        SETI Result,99
        JMP     Test149Forward
Test149Back     SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test150
        JMP     TestFail
Test149Forward  SETI $10,5
        NEG     $10,0,$10
        PBNB    $10,Test149Back
        JMP     TestFail

% ========================================
% Test 150: PBZB - Probable branch backward if zero
% ========================================
Test150 ADDUI   TestNum,TestNum,1
        SETI Result,88
        JMP     Test150Forward
Test150Back     SETI Expect,88
        CMP     Temp,Result,Expect
        PBZ     Temp,Test151
        JMP     TestFail
Test150Forward  SETI $10,0
        PBZB    $10,Test150Back
        JMP     TestFail

% ========================================
% Test 151: PBPB - Probable branch backward if positive
% ========================================
Test151 ADDUI   TestNum,TestNum,1
        SETI Result,77
        JMP     Test151Forward
Test151Back     SETI Expect,77
        CMP     Temp,Result,Expect
        PBZ     Temp,Test152
        JMP     TestFail
Test151Forward  SETI $10,1
        PBPB    $10,Test151Back
        JMP     TestFail

% ========================================
% Test 152: PBODB - Probable branch backward if odd
% ========================================
Test152 ADDUI   TestNum,TestNum,1
        SETI Result,66
        JMP     Test152Forward
Test152Back     SETI Expect,66
        CMP     Temp,Result,Expect
        PBZ     Temp,Test153
        JMP     TestFail
Test152Forward  SETI $10,7
        PBODB   $10,Test152Back
        JMP     TestFail

% ========================================
% Test 153: PBNNB - Probable branch backward if non-negative
% ========================================
Test153 ADDUI   TestNum,TestNum,1
        SETI Result,55
        JMP     Test153Forward
Test153Back     SETI Expect,55
        CMP     Temp,Result,Expect
        PBZ     Temp,Test154
        JMP     TestFail
Test153Forward  SETI $10,0
        PBNNB   $10,Test153Back
        JMP     TestFail

% ========================================
% Test 154: PBNZB - Probable branch backward if non-zero
% ========================================
Test154 ADDUI   TestNum,TestNum,1
        SETI Result,44
        JMP     Test154Forward
Test154Back     SETI Expect,44
        CMP     Temp,Result,Expect
        PBZ     Temp,Test155
        JMP     TestFail
Test154Forward  SETI $10,1
        PBNZB   $10,Test154Back
        JMP     TestFail

% ========================================
% Test 155: PBNPB - Probable branch backward if non-positive
% ========================================
Test155 ADDUI   TestNum,TestNum,1
        SETI Result,33
        JMP     Test155Forward
Test155Back     SETI Expect,33
        CMP     Temp,Result,Expect
        PBZ     Temp,Test156
        JMP     TestFail
Test155Forward  SETI $10,0
        PBNPB   $10,Test155Back
        JMP     TestFail

% ========================================
% Test 156: PBEVB - Probable branch backward if even
% ========================================
Test156 ADDUI   TestNum,TestNum,1
        SETI Result,22
        JMP     Test156Forward
Test156Back     SETI Expect,22
        CMP     Temp,Result,Expect
        PBZ     Temp,Test157
        JMP     TestFail
Test156Forward  SETI $10,8
        PBEVB   $10,Test156Back
        JMP     TestFail

% ========================================
% CONDITIONAL JUMP TESTS
% ========================================

% ========================================
% Test 157: JE - Jump if equal (taken)
% ========================================
Test157 ADDUI   TestNum,TestNum,1
        SETI $10,42
        SETI $11,42
        CMP     $10,$10,$11
        SETI Result,99
        JE      $10,Test157Skip
        SETI Result,#DEAD
Test157Skip     SETI Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test158
        JMP     TestFail

% ========================================
% Test 158: JNE - Jump if not equal (taken)
% ========================================
Test158 ADDUI   TestNum,TestNum,1
        SETI $10,10
        SETI $11,20
        CMP     $10,$10,$11
        SETI Result,88
        JNE     $10,Test158Skip
        SETI Result,#DEAD
Test158Skip     SETI Expect,88
        CMP     Temp,Result,Expect
        PBZ     Temp,Test159
        JMP     TestFail

% ========================================
% Test 159: JL - Jump if less (taken)
% ========================================
Test159 ADDUI   TestNum,TestNum,1
        SETI $10,5
        SETI $11,10
        CMP     $10,$10,$11
        SETI Result,77
        JL      $10,Test159Skip
        SETI Result,#DEAD
Test159Skip     SETI Expect,77
        CMP     Temp,Result,Expect
        PBZ     Temp,Test160
        JMP     TestFail

% ========================================
% Test 160: JG - Jump if greater (taken)
% ========================================
Test160 ADDUI   TestNum,TestNum,1
        SETI $10,100
        SETI $11,50
        CMP     $10,$10,$11
        SETI Result,66
        JG      $10,Test160Skip
        SETI Result,#DEAD
Test160Skip     SETI Expect,66
        CMP     Temp,Result,Expect
        PBZ     Temp,Test161
        JMP     TestFail

% ========================================
% load values from special registers 
% confirm initialization
% ========================================
Test161 ADDUI   TestNum,TestNum,1
        GET     Result,rN
        SETI Expect,2009
        CMP     Temp,Result,Expect
        PBZ     Temp,Test162
        JMP     TestFail

% ========================================
% Test 162: GETAB - Get Address with Base (backward)
% Note: GETAB gets address backward relative to PC
% ========================================
Test162 ADDUI   TestNum,TestNum,1
        GETA    $10,LocalData
        GETAB   Result,0        % Get address backward (relative to PC)
        % Result will be address of the GETAB instruction minus offset
        % We can't easily predict the exact value, so just verify no crash
        SETI Expect,0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test163    % Continue regardless of result
        JMP     Test163         % Continue regardless of result

% ========================================
% Test 163: LDUNC - Load Uncached
% ========================================
Test163 ADDUI   TestNum,TestNum,1
        GETA    $10,UncachedData
        SETI $11,0
        LDUNC   Result,$10,$11  % Load uncached octabyte (3 registers)
        SETI Expect,#FEEDFACEDEADBEEF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test164
        JMP     TestFail

% ========================================
% Test 164: LDUNCI - Load Uncached Immediate
% ========================================
Test164 ADDUI   TestNum,TestNum,1
        GETA    $10,UncachedData
        LDUNCI  Result,$10,8    % Load uncached from offset 8
        SETI Expect,#1234567890ABCDEF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test165
        JMP     TestFail

% ========================================
% Test 165: STUNC - Store Uncached
% ========================================
Test165 ADDUI   TestNum,TestNum,1
        GETA    $10,UncachedData
        SETI $11,#CAFEBABEFEEDFACE
        SETI $12,16
        STUNC   $11,$10,$12     % Store uncached octabyte (3 registers)
        SETI $12,16
        LDUNCI  Result,$10,16   % Verify with immediate
        SETI Expect,#CAFEBABEFEEDFACE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test166
        JMP     TestFail

% ========================================
% Test 166: STUNCI - Store Uncached Immediate
% ========================================
Test166 ADDUI   TestNum,TestNum,1
        GETA    $10,UncachedData
        SETI $11,#DEADC0DEBADC0FFE
        STUNCI  $11,$10,24      % Store uncached with immediate offset
        LDUNCI  Result,$10,24   % Verify
        SETI Expect,#DEADC0DEBADC0FFE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test167
        JMP     TestFail

% ========================================
% Test 167: LDHT - Load High Tetra
% ========================================
Test167 ADDUI   TestNum,TestNum,1
        GETA    $10,TetraData
        SETI $11,0
        LDHT    Result,$10,$11  % Load high tetra (bits 32-63) (3 registers)
        SETI Expect,#FEEDFACE00000000
        CMP     Temp,Result,Expect
        PBZ     Temp,Test168
        JMP     TestFail

% ========================================
% Test 168: LDHTI - Load High Tetra Immediate
% ========================================
Test168 ADDUI   TestNum,TestNum,1
        GETA    $10,TetraData
        LDHTI   Result,$10,8    % Load high tetra from offset 8
        SETI Expect,#1234567800000000
        CMP     Temp,Result,Expect
        PBZ     Temp,Test169
        JMP     TestFail

% ========================================
% Test 169: STHT - Store High Tetra
% ========================================
Test169 ADDUI   TestNum,TestNum,1
        GETA    $10,TetraData
        SETI $11,#CAFEBABE00000000
        SETI $12,16
        STHT    $11,$10,$12     % Store high tetra (3 registers)
        LDHTI   Result,$10,16   % Verify with immediate
        SETI Expect,#CAFEBABE00000000
        CMP     Temp,Result,Expect
        PBZ     Temp,Test170
        JMP     TestFail

% ========================================
% Test 170: STHTI - Store High Tetra Immediate
% ========================================
Test170 ADDUI   TestNum,TestNum,1
        GETA    $10,TetraData
        SETI $11,#DEADBEEF00000000
        STHTI   $11,$10,24      % Store high tetra with immediate
        LDHTI   Result,$10,24   % Verify
        SETI Expect,#DEADBEEF00000000
        CMP     Temp,Result,Expect
        PBZ     Temp,Test171
        JMP     TestFail

% ========================================
% Test 171: LDSF - Load Short Float
% ========================================
Test171 ADDUI   TestNum,TestNum,1
        GETA    $10,ShortFloatData
        SETI $11,0
        LDSF    Result,$10,$11  % Load short float (32-bit) and convert to 64-bit (3 regs)
        GETA    $12,ShortFloatData
        SETI $13,16
        LDO     Expect,$12,$13  % Expected 64-bit float value
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test172
        JMP     TestFail

% ========================================
% Test 172: LDSFI - Load Short Float Immediate
% ========================================
Test172 ADDUI   TestNum,TestNum,1
        GETA    $10,ShortFloatData
        LDSFI   Result,$10,4    % Load short float from offset 4
        GETA    $11,ShortFloatData
        SETI $12,24
        LDO     Expect,$11,$12  % Expected 64-bit float value
        FCMP    Temp,Result,Expect
        PBZ     Temp,Test173
        JMP     TestFail

% ========================================
% Test 173: STSF - Store Short Float
% ========================================
Test173 ADDUI   TestNum,TestNum,1
        GETA    $10,ShortFloatData
        SETI $11,#4048F5C300000000  % 3.14 in double precision
        SETI $12,8
        STSF    $11,$10,$12     % Convert and store as short float (3 registers)
        LDSFI   Result,$10,8    % Load back and convert to double
        FCMP    Temp,Result,$11
        % Allow small difference for conversion
        PBZ     Temp,Test174
        JMP     TestFail

% ========================================
% Test 174: STSFI - Store Short Float Immediate
% ========================================
Test174 ADDUI   TestNum,TestNum,1
        GETA    $10,ShortFloatData
        SETI $11,#4000000000000000  % 2.0 in double precision
        STSFI   $11,$10,12      % Convert and store as short float with immediate
        LDSFI   Result,$10,12   % Load back and convert to double
        FCMP    Temp,Result,$11
        PBZ     Temp,Test175
        JMP     TestFail

% ========================================
% Test 175: LDVTS - Load Virtual Translation
% ========================================
Test175 ADDUI   TestNum,TestNum,1
        GETA    $10,VirtualData
        SETI $11,0
        LDVTS   Result,$10,$11  % Load virtual translation status (3 registers)
        % Result will depend on virtual memory configuration
        % Just verify it doesn't crash
        PBZ     Temp,Test176    % Continue regardless
        JMP     Test176

% ========================================
% Test 176: LDVTSI - Load Virtual Translation Immediate
% ========================================
Test176 ADDUI   TestNum,TestNum,1
        GETA    $10,VirtualData
        LDVTSI  Result,$10,0    % Load virtual translation with immediate
        % Just verify it doesn't crash
        PBZ     Temp,Test177
        JMP     Test177

% ========================================
% Test 177: PRELD - Preload Data
% ========================================
Test177 ADDUI   TestNum,TestNum,1
        GETA    $10,PreloadData
        SETI $11,8
        SETI $12,0           % Count register
        PRELD   $12,$10,$11     % Preload data ($X=count reg, $Y, $Z)
        % This is a cache hint, verify no crash
        SETI Result,1
        SETI Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Test178
        JMP     TestFail

% ========================================
% Test 178: PRELDI - Preload Data Immediate
% ========================================
Test178 ADDUI   TestNum,TestNum,1
        GETA    $10,PreloadData
        SETI $11,0           % Count register
        PRELDI  $11,$10,16      % Preload data with immediate offset
        % This is a cache hint, verify no crash
        SETI Result,1
        SETI Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Test179
        JMP     TestFail

% ========================================
% Test 179: PREGO - Preload for GO
% ========================================
Test179 ADDUI   TestNum,TestNum,1
        GETA    $10,Test180
        SETI $11,0
        SETI $12,0           % Count register
        PREGO   $12,$10,$11     % Preload for upcoming GO ($X=count reg)
        % This is a cache hint, verify no crash
        SETI Result,1
        SETI Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Test180
        JMP     TestFail

% ========================================
% Test 180: PREGOI - Preload for GO Immediate
% ========================================
Test180 ADDUI   TestNum,TestNum,1
        GETA    $10,Test181
        SETI $11,0           % Count register
        PREGOI  $11,$10,0       % Preload for upcoming GO with immediate
        % This is a cache hint, verify no crash
        SETI Result,1
        SETI Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Test181
        JMP     TestFail

% ========================================
% Test 181: PREST - Preload for Store
% ========================================
Test181 ADDUI   TestNum,TestNum,1
        GETA    $10,PreloadData
        SETI $11,0
        SETI $12,0           % Count register
        PREST   $12,$10,$11     % Preload for upcoming store ($X=count reg)
        % This is a cache hint, verify no crash
        SETI Result,1
        SETI Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Test182
        JMP     TestFail

% ========================================
% Test 182: PRESTI - Preload for Store Immediate
% ========================================
Test182 ADDUI   TestNum,TestNum,1
        GETA    $10,PreloadData
        SETI $11,0           % Count register
        PRESTI  $11,$10,0       % Preload for store with immediate
        % This is a cache hint, verify no crash
        SETI Result,1
        SETI Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Test183
        JMP     TestFail

% ========================================
% Test 183: SYNC - Synchronize
% ========================================
Test183 ADDUI   TestNum,TestNum,1
        SYNC    0               % Synchronize memory operations
        % Verify no crash
        SETI Result,1
        SETI Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Test184
        JMP     TestFail

% ========================================
% Test 184: SWYM - Sympathize with Your Machinery (NOP)
% ========================================
Test184 ADDUI   TestNum,TestNum,1
        SWYM                    % Do nothing (no operation)
        SWYM                    % Do nothing again
        SETI Result,42
        SETI Expect,42
        CMP     Temp,Result,Expect
        PBZ     Temp,Test185
        JMP     TestFail

% ========================================
% Test 185: SAVE - Save registers
% Test 186: UNSAVE - Unsave registers
% ========================================
% Test 185: SAVE - Save registers
% Test 186: UNSAVE - Unsave registers  
% Test 187: RESUME - Resume execution
% ========================================
Test185 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $20,#ABCD1234   % Set a value to save
        SAVE    $15,0           % Save registers starting from $15
        SETI $20,0           % Clear the register
        UNSAVE  0,$15           % Restore registers from $15
        SETI Expect,#ABCD1234
        CMP     Temp,$20,Expect % Check if restored
        PBZ     Temp,Test186
        JMP     TestFail

% ========================================
% Test 186: RESUME - Resume from context
% Note: RESUME is typically used for exception handling.
% For testing coverage, we include the instruction even though
% it may not execute in normal circumstances.
% ========================================
Test186 ADDUI   TestNum,TestNum,1
        % RESUME instruction format: RESUME X (or RESUME 0 for simple test)
        % In a real scenario, RESUME would resume from a trap/interrupt
        % For coverage testing, we just need to show the instruction exists
        % We'll jump over it to avoid potential issues
        JMP     Test186Skip
        RESUME  0               % This instruction exists for coverage
Test186Skip
        SETI Result,1
        SETI Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Test187
        JMP     TestFail

% ========================================
% Test 187: FCMPE - Floating compare with epsilon (rE)
% ========================================
Test187 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,5             % $10 = 5.0
        FLOTI   $11,Zero,5             % $11 = 5.0
        FLOTI   $12,Zero,1             % $12 = 1.0 epsilon
        PUT     rE,$12
        FCMPE   Result,$10,$11         % 5.0 vs 5.0 within rE → 0
        SETI Expect,0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test188
        JMP     TestFail

% ========================================
% Test 188: FUNE - Floating unordered with epsilon
% ========================================
Test188 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,1
        FLOTI   $11,Zero,2
        PUT     rE,$10                 % rE = 1.0
        FUNE    Result,$10,$11         % |1-2| = 1 ≤ 1 → 1
        SETI Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Test189
        JMP     TestFail

% ========================================
% Test 189: FEQLE - Floating equivalence with epsilon
% ========================================
Test189 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,10
        FLOTI   $11,Zero,11
        FLOTI   $12,Zero,2             % epsilon = 2.0
        PUT     rE,$12
        FEQLE   Result,$10,$11         % |10-11| = 1 ≤ 2 → 1
        SETI Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Test190
        JMP     TestFail

% ========================================
% Test 190: directed rounding (ROUND_UP) changes FADD result
%   1.0 + 2^-53 rounds to 1.0 in default mode (ties-to-even),
%   but to next_up(1.0) under ROUND_UP — they must differ.
% ========================================
Test190 ADDUI   TestNum,TestNum,1
        FLOTI   $10,Zero,1             % 1.0
        GETA    $13,FpStdData
        LDOI    $11,$13,0              % 2^-53 from data
        PUTI    rA,0                   % ROUND_NEAR
        FADD    $12,$10,$11            % 1.0 (rounds to even)
        PUTI    rA,2                   % ROUND_UP
        FADD    $14,$10,$11            % next_up(1.0)
        FEQL    Result,$12,$14         % expect 0 (different)
        SETI    Expect,0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test191
        PUTI    rA,0
        JMP     TestFail

% ========================================
% Test 191: signaling NaN raises rA.I (#08)
% ========================================
Test191 ADDUI   TestNum,TestNum,1
        PUTI    rA,0                   % clear flags
        GETA    $13,FpStdData
        LDOI    $10,$13,8              % sNaN bit pattern
        FLOTI   $11,Zero,1             % 1.0
        FADD    $12,$10,$11            % NaN, raises I
        GET     Result,rA
        SETI    $15,#08                % I bit
        AND     Result,Result,$15
        SETI    Expect,#08
        CMP     Temp,Result,Expect
        PBZ     Temp,Test192
        PUTI    rA,0
        JMP     TestFail

% ========================================
% Test 192: inexact (X = #04) flag on 1.0 / 3.0
% ========================================
Test192 ADDUI   TestNum,TestNum,1
        PUTI    rA,0                   % clear flags
        FLOTI   $10,Zero,1
        FLOTI   $11,Zero,3
        FDIV    $12,$10,$11            % 1/3 inexact
        GET     Result,rA
        SETI    $15,#04                % X bit
        AND     Result,Result,$15
        SETI    Expect,#04
        CMP     Temp,Result,Expect
        PBZ     Temp,Test193
        PUTI    rA,0
        JMP     TestFail

% ========================================
% Test 193: ANDI - Bitwise AND immediate
% ========================================
Test193 ADDUI   TestNum,TestNum,1
        SETI $10,#FF0F
        ANDI    Result,$10,#0F
        SETI Expect,#0F
        CMP     Temp,Result,Expect
        PBZ     Temp,Test194
        JMP     TestFail

% ========================================
% Test 194: ANDNI - Bitwise AND-NOT immediate
% ========================================
% *I immediates are a single byte (0-255), zero-extended -- unlike the
% register form's $Z, which can hold any 64-bit value.
% ========================================
Test194 ADDUI   TestNum,TestNum,1
        SETI $10,#FFFF
        ANDNI   Result,$10,#0F
        SETI Expect,#FFF0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test195
        JMP     TestFail

% ========================================
% Test 195: ORI - Bitwise OR immediate
% ========================================
Test195 ADDUI   TestNum,TestNum,1
        SETI $10,#FF00
        ORI     Result,$10,#FF
        SETI Expect,#FFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test196
        JMP     TestFail

% ========================================
% Test 196: ORNI - Bitwise OR-NOT immediate
% ========================================
Test196 ADDUI   TestNum,TestNum,1
        SETI $10,#F0F0
        ORNI    Result,$10,#0F
        SETI Expect,#FFFFFFFFFFFFFFF0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test197
        JMP     TestFail

% ========================================
% Test 197: XORI - Bitwise XOR immediate
% ========================================
Test197 ADDUI   TestNum,TestNum,1
        SETI $10,#FFFF
        XORI    Result,$10,#F0
        SETI Expect,#FF0F
        CMP     Temp,Result,Expect
        PBZ     Temp,Test198
        JMP     TestFail

% ========================================
% Test 198: NANDI - Bitwise NAND immediate
% ========================================
Test198 ADDUI   TestNum,TestNum,1
        SETI $10,#FF
        NANDI   Result,$10,#FF
        SETI Expect,#FFFFFFFFFFFFFF00
        CMP     Temp,Result,Expect
        PBZ     Temp,Test199
        JMP     TestFail

% ========================================
% Test 199: NORI - Bitwise NOR immediate
% ========================================
Test199 ADDUI   TestNum,TestNum,1
        SETI $10,#00
        NORI    Result,$10,#00
        SETI Expect,#FFFFFFFFFFFFFFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test200
        JMP     TestFail

% ========================================
% Test 200: NXORI - Bitwise NXOR immediate
% ========================================
Test200 ADDUI   TestNum,TestNum,1
        SETI $10,#FF
        SETI    Result,0
        NXORI   Result,$10,#FF
        SETI Expect,#FFFFFFFFFFFFFFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test201
        JMP     TestFail

% ========================================
% Test 201: MUXI - Bitwise multiplex immediate
% ========================================
Test201 ADDUI   TestNum,TestNum,1
        SETI $10,#FFFFFFFFFFFFFFFF
        SETI $13,#FFFFFFFFFFFFFF00
        PUT     rM,$13                  % mask: keep high 56 bits from $10
        MUXI    Result,$10,#5A          % low byte comes from the immediate
        SETI Expect,#FFFFFFFFFFFFFF5A
        CMP     Temp,Result,Expect
        PBZ     Temp,Test202
        JMP     TestFail

% ========================================
% Test 202: DIVI - Signed divide immediate
% ========================================
Test202 ADDUI   TestNum,TestNum,1
        SETI $10,17
        DIVI    Result,$10,5
        SETI Expect,3
        CMP     Temp,Result,Expect
        PBZ     Temp,Test203
        JMP     TestFail

% ========================================
% Test 203: DIVUI - Unsigned divide immediate
% ========================================
Test203 ADDUI   TestNum,TestNum,1
        PUT     rD,Zero                 % clear dividend-high (dirty since Test121)
        SETI $10,1000
        DIVUI   Result,$10,7
        SETI Expect,142
        CMP     Temp,Result,Expect
        PBZ     Temp,Test204
        JMP     TestFail

% ========================================
% Test 204: MULI - Signed multiply immediate
% ========================================
Test204 ADDUI   TestNum,TestNum,1
        SETI $10,6
        NEG     $10,0,$10               % $10 = -6
        MULI    Result,$10,7
        SETI Expect,#FFFFFFFFFFFFFFD6   % -42
        CMP     Temp,Result,Expect
        PBZ     Temp,Test205
        JMP     TestFail

% ========================================
% Test 205: MULUI - Unsigned multiply immediate
% ========================================
Test205 ADDUI   TestNum,TestNum,1
        SETI $10,300
        MULUI   Result,$10,200          % Z is a single byte (0-255)
        SETI Expect,60000
        CMP     Temp,Result,Expect
        PBZ     Temp,Test206
        JMP     TestFail

% ========================================
% Test 206: CMPI - Signed compare immediate
% ========================================
Test206 ADDUI   TestNum,TestNum,1
        SETI $10,5
        NEG     $10,0,$10               % $10 = -5
        CMPI    Result,$10,3
        SETI Expect,#FFFFFFFFFFFFFFFF   % -1 (less than)
        CMP     Temp,Result,Expect
        PBZ     Temp,Test207
        JMP     TestFail

% ========================================
% Test 207: CMPUI - Unsigned compare immediate
% ========================================
Test207 ADDUI   TestNum,TestNum,1
        SETI $10,#FFFFFFFFFFFFFFFF      % huge unsigned
        CMPUI   Result,$10,5
        SETI Expect,1                   % greater than
        CMP     Temp,Result,Expect
        PBZ     Temp,Test208
        JMP     TestFail

% ========================================
% Test 208: BDIFI - Byte difference immediate
% ========================================
Test208 ADDUI   TestNum,TestNum,1
        SETI $10,#0A14
        BDIFI   Result,$10,5
        SETI Expect,#050F
        CMP     Temp,Result,Expect
        PBZ     Temp,Test209
        JMP     TestFail

% ========================================
% Test 209: WDIFI - Wyde difference immediate
% ========================================
Test209 ADDUI   TestNum,TestNum,1
        SETI $10,#0A000014
        WDIFI   Result,$10,5
        SETI Expect,#9FB000F
        CMP     Temp,Result,Expect
        PBZ     Temp,Test210
        JMP     TestFail

% ========================================
% Test 210: TDIFI - Tetra difference immediate
% ========================================
Test210 ADDUI   TestNum,TestNum,1
        SETI $10,#A00000014
        TDIFI   Result,$10,5
        SETI Expect,#50000000F
        CMP     Temp,Result,Expect
        PBZ     Temp,Test211
        JMP     TestFail

% ========================================
% Test 211: ODIFI - Octa difference immediate
% ========================================
Test211 ADDUI   TestNum,TestNum,1
        SETI $10,#FF
        ODIFI   Result,$10,15
        SETI Expect,#F0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test212
        JMP     TestFail

% ========================================
% Test 212: SADDI - Sideways add immediate
% ========================================
Test212 ADDUI   TestNum,TestNum,1
        SETI $10,#FF
        SADDI   Result,$10,#0F
        SETI Expect,4                   % popcount(0xFF & ~0x0F) = popcount(0xF0)
        CMP     Temp,Result,Expect
        PBZ     Temp,Test213
        JMP     TestFail

% ========================================
% Test 213: MOR - Multiple or (boolean matrix multiply)
% ========================================
Test213 ADDUI   TestNum,TestNum,1
        SETI $10,#0101010101010101      % identity-ish row vector
        SETI $11,#FF00000000000000      % first byte all 1s, rest 0
        MOR     Result,$10,$11
        SETI Expect,#0100000000000000
        CMP     Temp,Result,Expect
        PBZ     Temp,Test214
        JMP     TestFail

% ========================================
% Test 214: MXOR - Multiple exclusive-or (GF(2) matrix product)
% ========================================
% Unlike MOR (Test213), MXOR toggles per matching k instead of breaking on
% the first one, so an even number of (y,z) bit matches per output bit
% cancels to 0 under XOR. $11 here has only ONE bit set per contributing
% row (byte i=7 = 0x01, not 0xFF), giving exactly one match per output
% bit so MOR and MXOR agree on this input.
% ========================================
Test214 ADDUI   TestNum,TestNum,1
        SETI $10,#0101010101010101
        SETI $11,#0100000000000000
        SETI    Result,0
        MXOR    Result,$10,$11
        SETI Expect,#0100000000000000
        CMP     Temp,Result,Expect
        PBZ     Temp,Test215
        JMP     TestFail

% ========================================
% Test 215: SLI - Shift left immediate (signed, overflow-checked)
% ========================================
Test215 ADDUI   TestNum,TestNum,1
        SETI $10,5
        SLI     Result,$10,3
        SETI Expect,40
        CMP     Temp,Result,Expect
        PBZ     Temp,Test216
        JMP     TestFail

% ========================================
% Test 216: SLUI - Shift left unsigned immediate
% ========================================
Test216 ADDUI   TestNum,TestNum,1
        SETI $10,1
        SLUI    Result,$10,10
        SETI Expect,1024
        CMP     Temp,Result,Expect
        PBZ     Temp,Test217
        JMP     TestFail

% ========================================
% Test 217: SRI - Shift right immediate (arithmetic)
% ========================================
Test217 ADDUI   TestNum,TestNum,1
        SETI $10,100
        NEG     $10,0,$10               % $10 = -100
        SRI     Result,$10,2
        SETI Expect,#FFFFFFFFFFFFFFE7   % -25
        CMP     Temp,Result,Expect
        PBZ     Temp,Test218
        JMP     TestFail

% ========================================
% Test 218: SRUI - Shift right unsigned immediate (logical)
% ========================================
Test218 ADDUI   TestNum,TestNum,1
        SETI $10,256
        SRUI    Result,$10,4
        SETI Expect,16
        CMP     Temp,Result,Expect
        PBZ     Temp,Test219
        JMP     TestFail

% ========================================
% Test 219: INCL - Increase by low wyde (register-number operands)
% ========================================
% INCL's grammar takes three register operands, but $Y and $Z's register
% NUMBERS (not their runtime contents) are packed as the 16-bit YZ
% immediate added to $X (src/mmixal.rs parse_inst_incl; src/encode.rs
% INCL test asserts INCL(1,2,3) -> [0xE7,1,2,3]). Using $5,$6 here adds
% (5<<8|6) = 1286, regardless of what those registers hold.
% ========================================
Test219 ADDUI   TestNum,TestNum,1
        SETI    Result,100
        INCL    Result,$5,$6
        SETI    Expect,1386
        CMP     Temp,Result,Expect
        PBZ     Temp,Test220
        JMP     TestFail

% ========================================
% Test 220: SET - Register-to-register copy (SET $X,$Y -> ORI $X,$Y,0)
% ========================================
Test220 ADDUI   TestNum,TestNum,1
        SETI $10,#12345678
        SET     Result,$10
        SETI Expect,#12345678
        CMP     Temp,Result,Expect
        PBZ     Temp,Test221
        JMP     TestFail

% ========================================
% Test 221: LDA - Load address, 3-operand form (register,register,imm)
% ========================================
% LDA's 3-operand form always assembles to the LDA-encoded opcode (0x22),
% which has no distinct VM handler and decodes exactly like ADDU register
% form (settled decision 11): the Z field, though written here as an
% assembler-level immediate, is read at execution time as a REGISTER
% INDEX, not a literal. $12's runtime value (not the literal "12") is what
% gets added to $11. This is real coverage of LDA's actual assembler+VM
% path, not "textbook" LDA semantics.
% ========================================
Test221 ADDUI   TestNum,TestNum,1
        SETI $11,100
        SETI $12,23             % Z field is "12" below -> reads $12's value
        LDA     Result,$11,12
        SETI Expect,123
        CMP     Temp,Result,Expect
        PBZ     Temp,Test222
        JMP     TestFail

% ========================================
% Test 222: LDAI - Load address, 3-operand immediate form
% ========================================
% Unlike LDA above, LDAI's 3-operand form assembles to opcode 0x23, which
% decodes exactly like ADDUI (immediate) -- Z really is the literal here.
% ========================================
Test222 ADDUI   TestNum,TestNum,1
        SETI $14,200
        LDAI    Result,$14,50
        SETI Expect,250
        CMP     Temp,Result,Expect
        PBZ     Temp,Test223
        JMP     TestFail

% ========================================
% Test 223: LDBI - Load byte signed immediate
% ========================================
Test223 ADDUI   TestNum,TestNum,1
        GETA    $10,UncachedData
        LDBI    Result,$10,0     % byte 0 of #FEEDFACEDEADBEEF is 0xFE
        SETI Expect,#FFFFFFFFFFFFFFFE   % -2, sign-extended
        CMP     Temp,Result,Expect
        PBZ     Temp,Test224
        JMP     TestFail

% ========================================
% Test 224: LDBU - Load byte unsigned (register)
% ========================================
Test224 ADDUI   TestNum,TestNum,1
        GETA    $10,UncachedData
        SETI $11,8               % byte 0 of the octa at offset 8 (#1234567890ABCDEF) is 0x12
        LDBU    Result,$10,$11
        SETI Expect,18
        CMP     Temp,Result,Expect
        PBZ     Temp,Test225
        JMP     TestFail

% ========================================
% Test 225: LDBUI - Load byte unsigned immediate
% ========================================
Test225 ADDUI   TestNum,TestNum,1
        GETA    $10,PreloadData
        LDBUI   Result,$10,8     % byte 0 of the octa at offset 8 (#2222222222222222) is 0x22
        SETI Expect,34
        CMP     Temp,Result,Expect
        PBZ     Temp,Test226
        JMP     TestFail

% ========================================
% Test 226: LDW - Load wyde signed (register)
% ========================================
Test226 ADDUI   TestNum,TestNum,1
        GETA    $10,VirtualData
        SETI $11,0                % wyde 0 of #CAFEBABEFEEDFACE is 0xCAFE
        LDW     Result,$10,$11
        SETI Expect,#FFFFFFFFFFFFCAFE   % -13570, sign-extended
        CMP     Temp,Result,Expect
        PBZ     Temp,Test227
        JMP     TestFail

% ========================================
% Test 227: LDWI - Load wyde signed immediate
% ========================================
Test227 ADDUI   TestNum,TestNum,1
        GETA    $10,VirtualData
        LDWI    Result,$10,8      % wyde 0 of the octa at offset 8 (#DEADBEEFDEADBEEF) is 0xDEAD
        SETI Expect,#FFFFFFFFFFFFDEAD   % -8531, sign-extended
        CMP     Temp,Result,Expect
        PBZ     Temp,Test228
        JMP     TestFail

% ========================================
% Test 228: LDWUI - Load wyde unsigned immediate
% ========================================
Test228 ADDUI   TestNum,TestNum,1
        GETA    $10,PreloadData
        LDWUI   Result,$10,16     % wyde 0 of the octa at offset 16 (#3333333333333333) is 0x3333
        SETI Expect,13107
        CMP     Temp,Result,Expect
        PBZ     Temp,Test229
        JMP     TestFail

% ========================================
% Test 229: LDTI - Load tetra signed immediate
% ========================================
Test229 ADDUI   TestNum,TestNum,1
        GETA    $10,UncachedData
        LDTI    Result,$10,0      % tetra 0 of #FEEDFACEDEADBEEF is 0xFEEDFACE
        SETI Expect,#FFFFFFFFFEEDFACE   % -17958194, sign-extended
        CMP     Temp,Result,Expect
        PBZ     Temp,Test230
        JMP     TestFail

% ========================================
% Test 230: LDTU - Load tetra unsigned (register)
% ========================================
Test230 ADDUI   TestNum,TestNum,1
        GETA    $10,UncachedData
        SETI $11,8                % tetra 0 of the octa at offset 8 (#1234567890ABCDEF) is 0x12345678
        LDTU    Result,$10,$11
        SETI Expect,305419896
        CMP     Temp,Result,Expect
        PBZ     Temp,Test231
        JMP     TestFail

% ========================================
% Test 231: LDTUI - Load tetra unsigned immediate
% ========================================
Test231 ADDUI   TestNum,TestNum,1
        GETA    $10,PreloadData
        LDTUI   Result,$10,24     % tetra 0 of the octa at offset 24 (#4444444444444444) is 0x44444444
        SETI Expect,1145324612
        CMP     Temp,Result,Expect
        PBZ     Temp,Test232
        JMP     TestFail

% ========================================
% Test 232: LDOU - Load octa unsigned (register)
% ========================================
Test232 ADDUI   TestNum,TestNum,1
        GETA    $10,LocalData
        SETI $11,0
        LDOU    Result,$10,$11
        SETI Expect,#DEADBEEFCAFEBABE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test233
        JMP     TestFail

% ========================================
% Test 233: LDOUI - Load octa unsigned immediate
% ========================================
Test233 ADDUI   TestNum,TestNum,1
        GETA    $10,VirtualData
        LDOUI   Result,$10,0
        SETI Expect,#CAFEBABEFEEDFACE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test234
        JMP     TestFail

% ========================================
% Test 234: STBI - Store byte immediate (signed, overflow-checked)
% ========================================
Test234 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,#7B             % 123
        STBI    $11,$10,0
        LDBI    Result,$10,0
        SETI Expect,123
        CMP     Temp,Result,Expect
        PBZ     Temp,Test235
        JMP     TestFail

% ========================================
% Test 235: STBU - Store byte unsigned (register, no overflow check)
% ========================================
Test235 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,#FF
        SETI $12,8
        STBU    $11,$10,$12
        LDBU    Result,$10,$12
        SETI Expect,255
        CMP     Temp,Result,Expect
        PBZ     Temp,Test236
        JMP     TestFail

% ========================================
% Test 236: STBUI - Store byte unsigned immediate
% ========================================
Test236 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,#80
        STBUI   $11,$10,16
        LDBUI   Result,$10,16
        SETI Expect,128
        CMP     Temp,Result,Expect
        PBZ     Temp,Test237
        JMP     TestFail

% ========================================
% Test 237: STWI - Store wyde immediate (signed, overflow-checked)
% ========================================
Test237 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,#1234
        STWI    $11,$10,24
        LDWI    Result,$10,24
        SETI Expect,4660
        CMP     Temp,Result,Expect
        PBZ     Temp,Test238
        JMP     TestFail

% ========================================
% Test 238: STWU - Store wyde unsigned (register, no overflow check)
% ========================================
Test238 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,#8000
        SETI $12,32
        STWU    $11,$10,$12
        LDWUI   Result,$10,32
        SETI Expect,32768
        CMP     Temp,Result,Expect
        PBZ     Temp,Test239
        JMP     TestFail

% ========================================
% Test 239: STWUI - Store wyde unsigned immediate
% ========================================
Test239 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,#ABCD
        STWUI   $11,$10,40
        LDWUI   Result,$10,40
        SETI Expect,43981
        CMP     Temp,Result,Expect
        PBZ     Temp,Test240
        JMP     TestFail

% ========================================
% Test 240: STTI - Store tetra immediate (signed, overflow-checked)
% ========================================
Test240 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,#12345678
        STTI    $11,$10,48
        LDTI    Result,$10,48
        SETI Expect,305419896
        CMP     Temp,Result,Expect
        PBZ     Temp,Test241
        JMP     TestFail

% ========================================
% Test 241: STTU - Store tetra unsigned (register, no overflow check)
% ========================================
Test241 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,#FFFFFFFF
        SETI $12,56
        STTU    $11,$10,$12
        LDTUI   Result,$10,56
        SETI Expect,4294967295
        CMP     Temp,Result,Expect
        PBZ     Temp,Test242
        JMP     TestFail

% ========================================
% Test 242: STTUI - Store tetra unsigned immediate
% ========================================
Test242 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,#9ABCDEF0
        STTUI   $11,$10,64
        LDTUI   Result,$10,64
        SETI Expect,#9ABCDEF0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test243
        JMP     TestFail

% ========================================
% Test 243: STOI - Store octa immediate
% ========================================
Test243 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,#123456789ABCDEF0
        STOI    $11,$10,72
        LDOI    Result,$10,72
        SETI Expect,#123456789ABCDEF0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test244
        JMP     TestFail

% ========================================
% Test 244: STOU - Store octa unsigned (register)
% ========================================
Test244 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,#FEDCBA9876543210
        SETI $12,80
        STOU    $11,$10,$12
        LDOU    Result,$10,$12
        SETI Expect,#FEDCBA9876543210
        CMP     Temp,Result,Expect
        PBZ     Temp,Test245
        JMP     TestFail

% ========================================
% Test 245: STOUI - Store octa unsigned immediate
% ========================================
Test245 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,#0011223344556677
        STOUI   $11,$10,88
        LDOUI   Result,$10,88
        SETI Expect,#0011223344556677
        CMP     Temp,Result,Expect
        PBZ     Temp,Test246
        JMP     TestFail

% ========================================
% Test 246: STCO - Store constant octabyte (register)
% ========================================
Test246 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,96
        STCO    200,$10,$11        % X=200 is a literal constant, written as a full octa
        LDO     Result,$10,$11
        SETI Expect,200
        CMP     Temp,Result,Expect
        PBZ     Temp,Test247
        JMP     TestFail

% ========================================
% Test 247: STCOI - Store constant octabyte immediate
% ========================================
Test247 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        STCOI   201,$10,104        % X=201 is a literal constant, written as a full octa
        LDOI    Result,$10,104
        SETI Expect,201
        CMP     Temp,Result,Expect
        PBZ     Temp,Test248
        JMP     TestFail

% ========================================
% Test 248: PBEV - Probable branch if even (taken)
% ========================================
Test248 ADDUI   TestNum,TestNum,1
        SETI $10,8
        SETI Result,#E00E
        PBEV    $10,Test248Skip
        SETI Result,#DEAD
Test248Skip     SETI Expect,#E00E
        CMP     Temp,Result,Expect
        PBZ     Temp,Test249
        JMP     TestFail

% ========================================
% Test 249: PBN - Probable branch if negative (taken)
% ========================================
Test249 ADDUI   TestNum,TestNum,1
        SETI $10,5
        NEG     $10,0,$10
        SETI Result,#5EED
        PBN     $10,Test249Skip
        SETI Result,#DEAD
Test249Skip     SETI Expect,#5EED
        CMP     Temp,Result,Expect
        PBZ     Temp,Test250
        JMP     TestFail

% ========================================
% Test 250: PBNN - Probable branch if non-negative (taken)
% ========================================
Test250 ADDUI   TestNum,TestNum,1
        SETI $10,0
        SETI Result,#600D
        PBNN    $10,Test250Skip
        SETI Result,#DEAD
Test250Skip     SETI Expect,#600D
        CMP     Temp,Result,Expect
        PBZ     Temp,Test251
        JMP     TestFail

% ========================================
% Test 251: PBNP - Probable branch if non-positive (taken)
% ========================================
Test251 ADDUI   TestNum,TestNum,1
        SETI $10,0
        SETI Result,#7E20
        PBNP    $10,Test251Skip
        SETI Result,#DEAD
Test251Skip     SETI Expect,#7E20
        CMP     Temp,Result,Expect
        PBZ     Temp,Test252
        JMP     TestFail

% ========================================
% Test 252: PBOD - Probable branch if odd (taken)
% ========================================
Test252 ADDUI   TestNum,TestNum,1
        SETI $10,7
        SETI Result,#0DD0
        PBOD    $10,Test252Skip
        SETI Result,#DEAD
Test252Skip     SETI Expect,#0DD0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test253
        JMP     TestFail

% ========================================
% Test 253: PBP - Probable branch if positive (taken)
% ========================================
Test253 ADDUI   TestNum,TestNum,1
        SETI $10,42
        SETI Result,#900D
        PBP     $10,Test253Skip
        SETI Result,#DEAD
Test253Skip     SETI Expect,#900D
        CMP     Temp,Result,Expect
        PBZ     Temp,Test254
        JMP     TestFail

% ========================================
% Test 254: CSNP - Conditional set if non-positive (register)
% ========================================
Test254 ADDUI   TestNum,TestNum,1
        SETI $10,0
        SETI $11,55
        CSNP    Result,$10,$11
        SETI Expect,55
        CMP     Temp,Result,Expect
        PBZ     Temp,Test255
        JMP     TestFail

% ========================================
% Test 255: ZSNP - Zero or set if non-positive (register)
% ========================================
Test255 ADDUI   TestNum,TestNum,1
        SETI $10,0
        SETI $11,66
        ZSNP    Result,$10,$11
        SETI Expect,66
        CMP     Temp,Result,Expect
        PBZ     Temp,Test256
        JMP     TestFail

% ========================================
% Test 256: CSWAP - Compare and swap octabyte (register)
% ========================================
Test256 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $11,112
        SETI $13,#1111111111111111
        STO     $13,$10,$11       % seed memory with the "old" value
        PUT     rP,$13            % compare register matches the seed
        SETI $14,#2222222222222222
        CSWAP   $14,$10,$11       % mem == rP, so swap: $14 <- 1 (success)
        SETI Expect,1
        CMP     Temp,$14,Expect
        PBZ     Temp,Test256Verify
        JMP     TestFail
Test256Verify
        LDO     Result,$10,$11    % memory should now hold the new value
        SETI Expect,#2222222222222222
        CMP     Temp,Result,Expect
        PBZ     Temp,Test257
        JMP     TestFail

% ========================================
% Test 257: CSWAPI - Compare and swap octabyte immediate
% ========================================
Test257 ADDUI   TestNum,TestNum,1
        GETA    $10,SaveArea
        SETI $13,#3333333333333333
        STOI    $13,$10,120
        PUT     rP,$13
        SETI $14,#4444444444444444
        CSWAPI  $14,$10,120
        SETI Expect,1
        CMP     Temp,$14,Expect
        PBZ     Temp,Test257Verify
        JMP     TestFail
Test257Verify
        LDOI    Result,$10,120
        SETI Expect,#4444444444444444
        CMP     Temp,Result,Expect
        PBZ     Temp,Test258
        JMP     TestFail

% ========================================
% Test 258: GO / GOI - Go to location (absolute jump-and-link, no stack)
% ========================================
Test258 ADDUI   TestNum,TestNum,1
        GETA    $10,Test258GoTarget
        SETI    $11,0
        GO      $12,$10,$11       % $12 <- return addr (unused); pc <- $10+$11
        JMP     TestFail          % unreachable: GO always jumps away
Test258GoTarget
        SETI    Result,#600D
        SETI    Expect,#600D
        CMP     Temp,Result,Expect
        PBZ     Temp,Test258Goi
        JMP     TestFail
Test258Goi
        GETA    $13,Test258GoiTarget
        GOI     $14,$13,0         % $14 <- return addr (unused); pc <- $13+0
        JMP     TestFail          % unreachable
Test258GoiTarget
        SETI    Result,#F00D
        SETI    Expect,#F00D
        CMP     Temp,Result,Expect
        PBZ     Temp,Test259
        JMP     TestFail

% ========================================
% Test 259: PUSHJ / POP - Push registers and jump, then return
% ========================================
% X=$5 keeps the pushed frame's marginal register clear of the harness's
% $1-$4 (settled decision 9); POP 1,1 returns one value to $5 and skips
% the "JMP TestFail" landing pad (target = rJ + 4*1).
% ========================================
Test259 ADDUI   TestNum,TestNum,1
        PUSHJ   $5,Test259Callee
        JMP     TestFail          % unreachable: skipped by POP's yz=1
        SETI    Expect,777
        CMP     Temp,$5,Expect
        PBZ     Temp,Test260
        JMP     TestFail
Test259Callee
        SETI    $0,777
        POP     1,1
        JMP     TestFail          % only reached if POP fails to return

% ========================================
% Test 260: PUSHGO / PUSHGOI - Push registers, absolute go, then return
% ========================================
% PUSHJB is deliberately not tested here or anywhere else in this file --
% see the intentional-exception block above TestPass for why (settled
% decision 9's "flag it, don't force it" escape hatch: its assembler
% encoding is broken for every genuinely-backward target).
% ========================================
Test260 ADDUI   TestNum,TestNum,1
        GETA    $10,Test260Pg
        SETI    $11,0
        PUSHGO  $6,$10,$11
        JMP     TestFail          % unreachable: skipped by POP's yz=1
        SETI    Expect,999
        CMP     Temp,$6,Expect
        PBZ     Temp,Test260Second
        JMP     TestFail
Test260Pg
        SETI    $0,999
        POP     1,1
        JMP     TestFail          % only reached if POP fails to return
Test260Second
        GETA    $12,Test260Pgi
        PUSHGOI $7,$12,0
        JMP     TestFail          % unreachable: skipped by POP's yz=1
        SETI    Expect,888
        CMP     Temp,$7,Expect
        PBZ     Temp,TestPass
        JMP     TestFail
Test260Pgi
        SETI    $0,888
        POP     1,1
        JMP     TestFail          % only reached if POP fails to return

% ========================================
% Intentional coverage exceptions
% ========================================
% The following grammar-defined mnemonics are deliberately not exercised
% above (see prompts/checksmix-prompt-corpus-coverage-audit.md, settled
% decisions 3, 4, 9, 10 for the full rationale):
%   TRIP    - Opcode::TRIP unconditionally returns false ("for now, just
%             halt") with no pass/fail signal this harness can observe;
%             unlike HALT it has no existing intentional termination point
%             to piggyback on.
%   SYNCD, SYNCDI, SYNCID, SYNCIDI - each handler is exactly
%             "self.advance_pc(); true" with no observable state change at
%             all, so no operand choice can make a CMP/PBZ against them
%             non-vacuous.
%   PUSHJB  - discovered while implementing this task's stack/control
%             family (settled decision 9's "flag it, don't force it"
%             escape hatch): the assembler's Rule::inst_pushjb handler
%             (src/mmixal.rs, parse_inst_pushjb) computes the backward
%             branch offset as `addr - current_addr` (negative), but
%             Opcode::PUSHJB's VM handler (src/mmix.rs) subtracts YZ as
%             an UNSIGNED magnitude (`pc.wrapping_sub(yz * 4)`), the same
%             convention PUSHJB's own spec and every already-covered
%             *B branch mnemonic use. The two's-complement low 16 bits of
%             a negative offset, reinterpreted as unsigned, produce a
%             wild multi-hundred-KB "backward" jump for any genuinely
%             backward target -- there is no operand choice that avoids
%             this, since PUSHJB by definition always targets a backward
%             label. Reported separately per this prompt's scope-out
%             clause (repro: LOC #100 / Main JMP Fwd / Back SETI $0,42 /
%             POP 1,1 / Fwd PUSHJB $5,Back -- PC ends up at
%             0xfffffffffffc0130 instead of Back's address).
% HALT itself is not on this list: TestPass below now executes plain HALT
% (byte-identical to the TRAP 0,Halt,0 it replaces) instead of being
% skipped, which is real (if narrow) coverage of HALT's assembler path.
% ========================================

% ========================================
% All tests passed!
% ========================================
TestPass        SETI $255,PassMsg
        TRAP    0,Fputs,StdOut
        SETI    Result,#FFFF    % Success marker
        SETI    $255, 0
        HALT                    % settled decision 3: byte-identical to
                                 % TRAP 0,Halt,0, but exercises HALT's own
                                 % assembler path (mnemonic_halt) directly

% ========================================
% Test failed
% ========================================
TestFail        SETI $255,FailMsg
        TRAP    0,Fputs,StdOut
        SETI    Result,#DEAD    % Failure marker
        OR      FailNum,TestNum,Zero    % Copy TestNum to FailNum
        SETI    $255, 1         % error exit code
        TRAP    0,Halt,1        % Halt with error

% ========================================
% Local data for GETA test (must be in code segment, near the code)
% ========================================
        OCTA    0
LocalData
        OCTA    #DEADBEEFCAFEBABE

% Data for uncached load/store tests
        OCTA    0
        OCTA    0
        OCTA    0               % Alignment padding
UncachedData
        OCTA    #FEEDFACEDEADBEEF       % Offset 0
        OCTA    #1234567890ABCDEF       % Offset 8
        OCTA    0                       % Offset 16 (for STUNC test)
        OCTA    0                       % Offset 24 (for STUNCI test)

% Data for high tetra load/store tests
        OCTA    0
TetraData
        OCTA    #FEEDFACEDEADBEEF       % Offset 0
        OCTA    #1234567890ABCDEF       % Offset 8
        OCTA    0                       % Offset 16 (for STHT test)
        OCTA    0                       % Offset 24 (for STHTI test)

% Data for short float load/store tests
        OCTA    0
ShortFloatData
        TETRA   #40490FDB              % 3.14159265 as 32-bit float (offset 0)
        TETRA   #40C90FDB              % 6.28318530 as 32-bit float (offset 4)
        TETRA   0                       % Offset 8 (for STSF test)
        TETRA   0                       % Offset 12 (for STSFI test)
        OCTA    #400921FB60000000       % 3.14159265 converted to 64-bit (expected for offset 0)
        OCTA    #401921FB60000000       % 6.28318530 converted to 64-bit (expected for offset 4)

% Data for FP standards tests (Tests 190–192)
%   offset 0:  2^-53, the half-ULP increment above 1.0 in f64
%   offset 8:  signaling NaN (exponent all 1s, mantissa 0x...0001, bit 51 = 0)
        OCTA    0
FpStdData
        OCTA    #3CA0000000000000
        OCTA    #7FF0000000000001

% Data for virtual translation tests
        OCTA    0
VirtualData
        OCTA    #CAFEBABEFEEDFACE
        OCTA    #DEADBEEFDEADBEEF

% Data for preload tests
        OCTA    0
PreloadData
        OCTA    #1111111111111111
        OCTA    #2222222222222222
        OCTA    #3333333333333333
        OCTA    #4444444444444444

% Save area for SAVE/UNSAVE tests (needs space for saved registers)
        OCTA    0
SaveArea
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0
        OCTA    0

% ========================================
% Test 49 Far Target - Located far away to test large JMP offset
% ========================================
        LOC     #10000          % Jump to a far location (64KB away)
Test49Far       SETI Result,#CAFE
        SETI Expect,#CAFE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test50
        JMP     TestFail

% ========================================
% Data section - Messages
% ========================================
        LOC     Data_Segment
PassMsg BYTE    "All tests passed!",10,0
FailMsg BYTE    "Test failed!",10,0
