	LOC	Data_Segment
	GREG	@
Text	BYTE	"Hello world!",13,0

	LOC	#100

Main	debug "Version 0.1: Hello World Example"	
	LDA	$0,Text
	TRAP	0,Fputs,StdOut
	TRAP	0,Halt,0
	END	Main