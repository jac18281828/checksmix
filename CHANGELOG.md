0.2.5 (2025-12-26)

* Fixed: PUSHJ/POP now correctly restore caller's rJ register in nested function calls
* Fixed: rG (global threshold register) now defaults to 32 per MMIX specification
* This enables proper execution of programs with nested subroutines and return values

0.2.4 (2025-12-25)

* support for pushj/pop

0.2.1 (2025-12-17)

* fix deployment

0.2.0 (2025-12-17)

* full mmix implementation
* massive refactor
* works - so many improvements

0.1.0 (2025-11-19)

* initial working version