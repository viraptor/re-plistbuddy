# re-plistbuddy

Reimplementation of macOS `PlistBuddy` and `plutil` utilities, aiming for bug-for-bug output compatibility with the originals. Uses Core Foundation FFI directly (no third-party plist crate) for exact serialization behavior.

Protected/specced by 368 tests (69 unit + 186 PlistBuddy integration + 113 plutil integration).

## Binaries

- **`PlistBuddy`** — drop-in replacement for `/usr/libexec/PlistBuddy`
- **`plutil`** — drop-in replacement for `/usr/bin/plutil`

## Known differences from the originals

### PlistBuddy

| Area | Difference | Detail |
|------|-----------|--------|
| Float text formatting | Edge-case rounding at extreme precision | For the specific value `999999999.999999` (IEEE 754 `0x41cdcd64fffffff8`), both `CFStringCreateWithFormat("%.6f")` and `snprintf("%.6f")` produce `999999999.999999`, but the original PlistBuddy displays `1000000000.000000`. The original likely uses a different internal representation or formatting path. This only affects values at the boundary of float64 precision where the 7th decimal digit is 0. |
| `-c` command buffer | No length limit | The original PlistBuddy silently truncates `-c` commands longer than ~4KB. |
| CF error messages | Wording varies by macOS version | Error messages from Core Foundation (e.g. for corrupted files) may differ in exact wording between macOS versions. For example: `"stream had too few bytes"` vs `"Cannot parse a NULL or zero-length data"` for empty files. |

### plutil

| Area | Difference | Detail |
|------|-----------|--------|
| `/dev/null` handling | Reads as empty | The original plutil reports a permission error when given `/dev/null` as input. Our implementation reads it successfully as an empty file (empty dictionary). |
| CF error messages | Wording varies by macOS version | Same as PlistBuddy — CF error message text may differ. |

### Both tools

All other behavior: commands, flags, errors, output formats, exit codes, stdout/stderr routing, key ordering, escape handling, interactive mode, symlink handling, file creation, date formatting (including timezone abbreviation), XML/binary/JSON/Swift/Obj-C output — has been verified to match the originals, compared output byte-for-byte against the system utilities.

Code almost entirely generated, based on behaviour of the original tools.
