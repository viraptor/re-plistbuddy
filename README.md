# PlistBuddy reimplementation

Open reimplementation of PlistBuddy behaviour. It's a bug-for-bug compatible
implementation of Apple's PlistBuddy tool. In scope for compatibility testing:

- console output format
- exit codes
- commandline arguements
- all operations
- resulting files, including all formatting
- bugs/weird behaviour (for example dates are always output in non-DST format, like the original tool does)

Over 200 tests validate the behaviour.

Code almost entirely generated.
