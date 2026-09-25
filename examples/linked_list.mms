% linked_list.mms -- walk a linked list and sum its nodes.
%
% The sum lands in $5 and is not printed: printing it would add another
% copy of the decimal-print routine for a value the final state dump
% already shows.

        % ----------------------------------------------------
        % Data segment: statically allocate three nodes
        % ----------------------------------------------------
        LOC     Data_Segment
        GREG    @
Node1   OCTA    1               % node1.value = 1
        OCTA    Node2           % node1.next = Node2
Node2   OCTA    2               % node2.value = 2
        OCTA    Node3           % node2.next = Node3
Node3   OCTA    3               % node3.value = 3
        OCTA    0               % node3.next = NULL

        % ----------------------------------------------------
        % Program segment
        % ----------------------------------------------------
        LOC     #100

        % ----------------------------------------------------
        % Registers used:
        %   $1 = head pointer
        %   $2 = current value
        %   $4 = temp
        %   $5 = sum
        %   $255 = zero, for the NULL compare (Zero IS $255)
        % ----------------------------------------------------

Zero    IS      $255

Main    SETI    $5,0                % sum = 0
        SETI    $255,0              % $255 = 0
        LDA     $1,Node1            % head = address of node1

        % -------- traversal: sum all values --------
Traverse
        CMP     $4,$1,Zero          % compare head to NULL
        BZ      $4,Done             % if equal (zero result), exit

        LDO     $2,$1,0             % load value from node
        ADD     $5,$5,$2            % sum += value

        LDOI    $1,$1,8             % move to next node (load from offset 8)
        JMP     Traverse

Done
        TRAP    0,Halt,0
