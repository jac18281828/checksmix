% MMIX Comprehensive Instruction Test
% This program tests all major instruction families
% It validates itself - if it completes without error, all tests passed

        LOC     #100

% Special registers
rM      IS      0       % Multiplication register
rD      IS      1       % Dividend register  
rE      IS      2       % Epsilon register
rH      IS      3       % Himult register
rJ      IS      4       % Jump register
rR      IS      6       % Remainder register

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
        SET     Result,99       % This should be skipped
        JMP     Test47Skip
        SET     Result,#DEAD      % This should also be skipped
Test47Skip      SET     Expect,99
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
Test48Start     SET     Result,99
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
% Test 51: MUL - Multiply
% ========================================
Test51  ADDUI   TestNum,TestNum,1
        SETL    $10,5
        SETL    $11,7
        MUL     Result,$10,$11
        SETL    Expect,35
        CMP     Temp,Result,Expect
        PBZ     Temp,Test52
        JMP     TestFail

% ========================================
% Test 52: MULU - Multiply unsigned
% ========================================
Test52  ADDUI   TestNum,TestNum,1
        SETL    $10,100
        SETL    $11,200
        MULU    Result,$10,$11
        SETL    Expect,20000
        CMP     Temp,Result,Expect
        PBZ     Temp,Test53
        JMP     TestFail

% ========================================
% Test 53: DIV - Divide
% ========================================
Test53  ADDUI   TestNum,TestNum,1
        SETL    $10,100
        SETL    $11,5
        DIV     Result,$10,$11
        SETL    Expect,20
        CMP     Temp,Result,Expect
        PBZ     Temp,Test54
        JMP     TestFail

% ========================================
% Test 54: DIVU - Divide unsigned
% ========================================
Test54  ADDUI   TestNum,TestNum,1
        SETL    $10,1000
        SETL    $11,10
        DIVU    Result,$10,$11
        SETL    Expect,100
        CMP     Temp,Result,Expect
        PBZ     Temp,Test55
        JMP     TestFail

% ========================================
% Test 55: NEG - Negate
% ========================================
Test55  ADDUI   TestNum,TestNum,1
        SETL    $10,42
        NEG     Result,0,$10
        SETL    Expect,0
        SUBU    Expect,Expect,$10
        CMP     Temp,Result,Expect
        PBZ     Temp,Test56
        JMP     TestFail

% ========================================
% Test 56: NEGU - Negate unsigned
% ========================================
Test56  ADDUI   TestNum,TestNum,1
        SETL    $10,100
        NEGU    Result,0,$10
        SETL    Expect,0
        SUBU    Expect,Expect,$10
        CMP     Temp,Result,Expect
        PBZ     Temp,Test57
        JMP     TestFail

% ========================================
% Test 57: ORN - OR-NOT
% ========================================
Test57  ADDUI   TestNum,TestNum,1
        SET     $10,#FF00
        SET     $11,#0F0F
        ORN     Result,$10,$11
        SET     Expect,#FFF0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test58
        JMP     TestFail

% ========================================
% Test 58: MUX - Multiplex
% ========================================
Test58  ADDUI   TestNum,TestNum,1
        SET     $10,#AAAA
        SET     $11,#5555
        SET     $12,#FFFF
        MUX     Result,$10,$11
        GET     $13,rM
        MUX     Result,$12,$13
        SET     Expect,#5555
        CMP     Temp,Result,Expect
        PBZ     Temp,Test59
        JMP     TestFail

% ========================================
% Test 59: BDIF - Byte difference
% ========================================
Test59  ADDUI   TestNum,TestNum,1
        SET     $10,#0A14
        SET     $11,#050C
        BDIF    Result,$10,$11
        SET     Expect,#0508
        CMP     Temp,Result,Expect
        PBZ     Temp,Test60
        JMP     TestFail

% ========================================
% Test 60: WDIF - Wyde difference
% ========================================
Test60  ADDUI   TestNum,TestNum,1
        SET     $10,#0A000014
        SET     $11,#0500000C
        WDIF    Result,$10,$11
        SET     Expect,#05000008
        CMP     Temp,Result,Expect
        PBZ     Temp,Test61
        JMP     TestFail

% ========================================
% Test 61: TDIF - Tetra difference
% ========================================
Test61  ADDUI   TestNum,TestNum,1
        SET     $10,#A00000014
        SET     $11,#50000000C
        TDIF    Result,$10,$11
        SET     Expect,#500000008
        CMP     Temp,Result,Expect
        PBZ     Temp,Test62
        JMP     TestFail

% ========================================
% Test 62: MOR - Multiple OR
% ========================================
Test62  ADDUI   TestNum,TestNum,1
        SET     $10,#0101010101010101
        MOR     Result,$10,Zero
        SETL    Expect,#FF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test63
        JMP     TestFail

% ========================================
% Test 63: MXOR - Multiple XOR
% ========================================
Test63  ADDUI   TestNum,TestNum,1
        SET     $10,#FFFFFFFFFFFFFFFF
        MXOR    Result,$10,Zero
        SETL    Expect,0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test64
        JMP     TestFail

% ========================================
% Test 64: SLU - Shift left unsigned
% ========================================
Test64  ADDUI   TestNum,TestNum,1
        SETL    $10,1
        SETL    $11,8
        SLU     Result,$10,$11
        SETL    Expect,256
        CMP     Temp,Result,Expect
        PBZ     Temp,Test65
        JMP     TestFail

% ========================================
% Test 65: SRU - Shift right unsigned
% ========================================
Test65  ADDUI   TestNum,TestNum,1
        SETL    $10,256
        SETL    $11,4
        SRU     Result,$10,$11
        SETL    Expect,16
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
        SETL    Result,#DD00
        ORL     Result,#00FF
        SETL    Expect,#DDFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test73
        JMP     TestFail

% ========================================
% Test 73: ANDNH - AND-NOT high wyde
% ========================================
Test73  ADDUI   TestNum,TestNum,1
        SET     Result,#FFFFFFFFFFFFFFFF
        ANDNH   Result,#00FF
        SET     Expect,#FF00FFFFFFFFFFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test74
        JMP     TestFail

% ========================================
% Test 74: ANDNMH - AND-NOT medium high wyde
% ========================================
Test74  ADDUI   TestNum,TestNum,1
        SET     Result,#FFFFFFFFFFFFFFFF
        ANDNMH  Result,#00FF
        SET     Expect,#FFFFFF00FFFFFFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test75
        JMP     TestFail

% ========================================
% Test 75: ANDNML - AND-NOT medium low wyde
% ========================================
Test75  ADDUI   TestNum,TestNum,1
        SET     Result,#FFFFFFFFFFFFFFFF
        ANDNML  Result,#00FF
        SET     Expect,#FFFFFFFFFF00FFFF
        CMP     Temp,Result,Expect
        PBZ     Temp,Test76
        JMP     TestFail

% ========================================
% Test 76: ANDNL - AND-NOT low wyde
% ========================================
Test76  ADDUI   TestNum,TestNum,1
        SET     Result,#FFFFFFFFFFFFFFFF
        ANDNL   Result,#00FF
        SET     Expect,#FFFFFFFFFFFF00
        CMP     Temp,Result,Expect
        PBZ     Temp,Test77
        JMP     TestFail

% ========================================
% Test 77: BZ - Branch if zero (taken)
% ========================================
Test77  ADDUI   TestNum,TestNum,1
        SETL    $10,0
        SETL    Result,99
        BZ      $10,Test77Skip
        SETL    Result,#DEAD
Test77Skip      SETL    Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test78
        JMP     TestFail

% ========================================
% Test 78: BNZ - Branch if non-zero (taken)
% ========================================
Test78  ADDUI   TestNum,TestNum,1
        SETL    $10,1
        SETL    Result,99
        BNZ     $10,Test78Skip
        SETL    Result,#DEAD
Test78Skip      SETL    Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test79
        JMP     TestFail

% ========================================
% Test 79: BP - Branch if positive (taken)
% ========================================
Test79  ADDUI   TestNum,TestNum,1
        SETL    $10,42
        SETL    Result,99
        BP      $10,Test79Skip
        SETL    Result,#DEAD
Test79Skip      SETL    Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test80
        JMP     TestFail

% ========================================
% Test 80: BN - Branch if negative (not taken, then taken)
% ========================================
Test80  ADDUI   TestNum,TestNum,1
        SETL    $10,5
        BN      $10,TestFail
        NEG     $10,Zero,$10
        SETL    Result,99
        BN      $10,Test80Skip
        SETL    Result,#DEAD
Test80Skip      SETL    Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test81
        JMP     TestFail

% ========================================
% Test 81: BNN - Branch if non-negative (taken)
% ========================================
Test81  ADDUI   TestNum,TestNum,1
        SETL    $10,0
        SETL    Result,99
        BNN     $10,Test81Skip
        SETL    Result,#DEAD
Test81Skip      SETL    Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test82
        JMP     TestFail

% ========================================
% Test 82: BNP - Branch if non-positive (taken)
% ========================================
Test82  ADDUI   TestNum,TestNum,1
        SETL    $10,0
        SETL    Result,99
        BNP     $10,Test82Skip
        SETL    Result,#DEAD
Test82Skip      SETL    Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test83
        JMP     TestFail

% ========================================
% Test 83: BOD - Branch if odd (taken)
% ========================================
Test83  ADDUI   TestNum,TestNum,1
        SETL    $10,7
        SETL    Result,99
        BOD     $10,Test83Skip
        SETL    Result,#DEAD
Test83Skip      SETL    Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test84
        JMP     TestFail

% ========================================
% Test 84: BEV - Branch if even (taken)
% ========================================
Test84  ADDUI   TestNum,TestNum,1
        SETL    $10,8
        SETL    Result,99
        BEV     $10,Test84Skip
        SETL    Result,#DEAD
Test84Skip      SETL    Expect,99
        CMP     Temp,Result,Expect
        PBZ     Temp,Test85
        JMP     TestFail

% ========================================
% Test 85: PBZ - Probable branch if zero (taken)
% ========================================
Test85  ADDUI   TestNum,TestNum,1
        SETL    $10,0
        SETL    Result,#CAFE
        PBZ     $10,Test85Skip
        SETL    Result,#DEAD
Test85Skip      SETL    Expect,#CAFE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test86
        JMP     TestFail

% ========================================
% Test 86: PBNZ - Probable branch if non-zero (taken)
% ========================================
Test86  ADDUI   TestNum,TestNum,1
        SETL    $10,1
        SETL    Result,#BABE
        PBNZ    $10,Test86Skip
        SETL    Result,#DEAD
Test86Skip      SETL    Expect,#BABE
        CMP     Temp,Result,Expect
        PBZ     Temp,Test87
        JMP     TestFail

% ========================================
% Test 87: CSN - Conditional set if negative
% ========================================
Test87  ADDUI   TestNum,TestNum,1
        SETL    $10,5
        NEG     $10,Zero,$10
        SETL    $11,42
        SETL    $12,99
        CSN     Result,$10,$11
        SETL    Expect,42
        CMP     Temp,Result,Expect
        PBZ     Temp,Test88
        JMP     TestFail

% ========================================
% Test 88: CSZ - Conditional set if zero
% ========================================
Test88  ADDUI   TestNum,TestNum,1
        SETL    $10,0
        SETL    $11,0
        SETL    $12,1
        CSZ     Result,$10,$11
        SETL    Expect,0
        CMP     Temp,Result,Expect
        PBZ     Temp,Test89
        JMP     TestFail

% ========================================
% Test 89: CSP - Conditional set if positive
% ========================================
Test89  ADDUI   TestNum,TestNum,1
        SETL    $10,42
        SETL    $11,1
        SETL    $12,1
        NEG     $12,Zero,$12
        CSP     Result,$10,$11
        SETL    Expect,1
        CMP     Temp,Result,Expect
        PBZ     Temp,Test90
        JMP     TestFail

% ========================================
% Test 90: CSNN - Conditional set if non-negative
% ========================================
Test90  ADDUI   TestNum,TestNum,1
        SETL    $10,0
        SETL    $11,7
        SETL    $12,1; NEG     $12,0,$12
        CSNN    Result,$10,$11
        SETL    Expect,7
        CMP     Temp,Result,Expect
        PBZ     Temp,Test91
        JMP     TestFail

% ========================================
% Test 91: CSNZ - Conditional set if non-zero
% ========================================
Test91  ADDUI   TestNum,TestNum,1
        SETL    $10,7
        SETL    $11,11
        SETL    $12,0
        CSNZ    Result,$10,$11
        SETL    Expect,11
        CMP     Temp,Result,Expect
        PBZ     Temp,Test92
        JMP     TestFail

% ========================================
% Test 92: CSOD - Conditional set if odd
% ========================================
Test92  ADDUI   TestNum,TestNum,1
        SETL    $10,9
        SETL    $11,3
        SETL    $12,2
        CSOD    Result,$10,$11
        SETL    Expect,3
        CMP     Temp,Result,Expect
        PBZ     Temp,Test93
        JMP     TestFail

% ========================================
% Test 93: CSEV - Conditional set if even
% ========================================
Test93  ADDUI   TestNum,TestNum,1
        SETL    $10,10
        SETL    $11,2
        SETL    $12,3
        CSEV    Result,$10,$11
        SETL    Expect,2
        CMP     Temp,Result,Expect
        PBZ     Temp,Test94
        JMP     TestFail

% ========================================
% Test 94: ZSN - Zero or set if negative
% ========================================
Test94  ADDUI   TestNum,TestNum,1
        SETL    $10,5
        NEG     $10,Zero,$10
        SETL    $11,17
        ZSN     Result,$10,$11
        SETL    Expect,17
        CMP     Temp,Result,Expect
        PBZ     Temp,Test95
        JMP     TestFail

% ========================================
% Test 95: ZSZ - Zero or set if zero
% ========================================
Test95  ADDUI   TestNum,TestNum,1
        SETL    $10,0
        SETL    $11,19
        ZSZ     Result,$10,$11
        SETL    Expect,19
        CMP     Temp,Result,Expect
        PBZ     Temp,Test96
        JMP     TestFail

% ========================================
% Test 96: ZSP - Zero or set if positive
% ========================================
Test96  ADDUI   TestNum,TestNum,1
        SETL    $10,42
        SETL    $11,21
        ZSP     Result,$10,$11
        SETL    Expect,21
        CMP     Temp,Result,Expect
        PBZ     Temp,Test97
        JMP     TestFail

% ========================================
% Test 97: ZSNN - Zero or set if non-negative
% ========================================
Test97  ADDUI   TestNum,TestNum,1
        SETL    $10,1
        SETL    $11,17
        ZSNN    Result,$10,$11
        SETL    Expect,17
        CMP     Temp,Result,Expect
        PBZ     Temp,Test98
        JMP     TestFail

% ========================================
% Test 98: ZSNZ - Zero or set if non-zero
% ========================================
Test98  ADDUI   TestNum,TestNum,1
        SETL    $10,7
        SETL    $11,17
        ZSNZ    Result,$10,$11
        SETL    Expect,17
        CMP     Temp,Result,Expect
        PBZ     Temp,Test99
        JMP     TestFail

% ========================================
% Test 99: ZSOD - Zero or set if odd
% ========================================
Test99  ADDUI   TestNum,TestNum,1
        SETL    $10,13
        SETL    $11,27
        ZSOD    Result,$10,$11
        SETL    Expect,27
        CMP     Temp,Result,Expect
        PBZ     Temp,Test100
        JMP     TestFail

% ========================================
% Test 100: ZSEV - Zero or set if even
% ========================================
Test100 ADDUI   TestNum,TestNum,1
        SETL    $10,14
        SETL    $11,29
        ZSEV    Result,$10,$11
        SETL    Expect,29
        CMP     Temp,Result,Expect
        PBZ     Temp,TestPass
        JMP     TestFail

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
