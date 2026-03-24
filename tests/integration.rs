use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn binary() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push("PlistBuddy");
    path
}

fn temp_plist(content: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tid = std::thread::current().id();
    let path = std::env::temp_dir().join(format!("re_pb_test_{tid:?}_{id}.plist"));
    fs::write(&path, content).unwrap();
    path
}

fn empty_dict_plist() -> PathBuf {
    temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict/>
</plist>"#,
    )
}

fn sample_plist() -> PathBuf {
    temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Name</key>
	<string>Test App</string>
	<key>Version</key>
	<integer>42</integer>
	<key>Enabled</key>
	<true/>
	<key>Rating</key>
	<real>3.14</real>
	<key>Icon</key>
	<data>AQIDBA==</data>
	<key>Tags</key>
	<array>
		<string>alpha</string>
		<string>beta</string>
	</array>
	<key>Nested</key>
	<dict>
		<key>Inner</key>
		<string>value</string>
	</dict>
</dict>
</plist>"#,
    )
}

fn root_array_plist() -> PathBuf {
    temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<string>first</string>
	<string>second</string>
</array>
</plist>"#,
    )
}

fn deep_plist() -> PathBuf {
    temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>a</key>
	<dict>
		<key>b</key>
		<dict>
			<key>c</key>
			<dict>
				<key>d</key>
				<string>deep</string>
			</dict>
		</dict>
	</dict>
</dict>
</plist>"#,
    )
}

fn special_keys_plist() -> PathBuf {
    temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>key with spaces</key>
	<string>spaced</string>
	<key>key&amp;special</key>
	<string>special</string>
	<key>emoji🎉</key>
	<string>party</string>
	<key></key>
	<string>empty key</string>
</dict>
</plist>"#,
    )
}

struct RunResult {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

fn run(args: &[&str]) -> RunResult {
    let output = Command::new(binary())
        .args(args)
        .output()
        .expect("failed to execute PlistBuddy");
    RunResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    }
}

fn run_c(cmd: &str, file: &Path) -> RunResult {
    run(&["-c", cmd, file.to_str().unwrap()])
}

fn run_multi_c(cmds: &[&str], file: &Path) -> RunResult {
    let mut args: Vec<&str> = Vec::new();
    for cmd in cmds {
        args.push("-c");
        args.push(cmd);
    }
    args.push(file.to_str().unwrap());
    run(&args)
}

// ============================================================
// CLI argument handling
// ============================================================

#[test]
fn no_args_prints_usage_to_stdout() {
    let r = run(&[]);
    assert!(r.stdout.starts_with("Usage: PlistBuddy"), "stdout: {}", r.stdout);
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 1);
}

#[test]
fn dash_h_no_file_prints_full_help() {
    let r = run(&["-h"]);
    assert!(r.stdout.contains("Command Format:"), "stdout: {}", r.stdout);
    assert!(r.stdout.contains("Entry Format:"));
    assert!(r.stdout.contains("Examples:"));
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 1);
}

#[test]
fn dash_h_with_file_prints_full_help() {
    let f = sample_plist();
    let r = run(&["-h", f.to_str().unwrap()]);
    assert!(r.stdout.contains("Command Format:"));
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);
}

// ============================================================
// Print command - stdout output
// ============================================================

#[test]
fn print_string_value() {
    let f = sample_plist();
    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "Test App\n");
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_integer_value() {
    let f = sample_plist();
    let r = run_c("Print :Version", &f);
    assert_eq!(r.stdout, "42\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_bool_true() {
    let f = sample_plist();
    let r = run_c("Print :Enabled", &f);
    assert_eq!(r.stdout, "true\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_real_value() {
    let f = sample_plist();
    let r = run_c("Print :Rating", &f);
    assert_eq!(r.stdout, "3.140000\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_array() {
    let f = sample_plist();
    let r = run_c("Print :Tags", &f);
    assert_eq!(r.stdout, "Array {\n    alpha\n    beta\n}\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_dict() {
    let f = sample_plist();
    let r = run_c("Print :Nested", &f);
    assert_eq!(r.stdout, "Dict {\n    Inner = value\n}\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_array_element() {
    let f = sample_plist();
    let r = run_c("Print :Tags:0", &f);
    assert_eq!(r.stdout, "alpha\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_nested_key() {
    let f = sample_plist();
    let r = run_c("Print :Nested:Inner", &f);
    assert_eq!(r.stdout, "value\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_data_outputs_raw_bytes() {
    let f = sample_plist();
    let r = run_c("Print :Icon", &f);
    assert_eq!(r.stdout.as_bytes(), b"\x01\x02\x03\x04\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_empty_entry_prints_whole_file() {
    let f = sample_plist();
    let r = run_c("Print", &f);
    assert!(r.stdout.starts_with("Dict {\n"));
    assert!(r.stdout.ends_with("}\n"));
    assert!(r.stdout.contains("Name = Test App"));
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_colon_entry_prints_whole_file() {
    let f = sample_plist();
    let r = run_c("Print :", &f);
    assert!(r.stdout.starts_with("Dict {\n"));
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_without_leading_colon() {
    let f = sample_plist();
    let r = run_c("Print Name", &f);
    assert_eq!(r.stdout, "Test App\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_deep_path() {
    let f = deep_plist();
    let r = run_c("Print :a:b:c:d", &f);
    assert_eq!(r.stdout, "deep\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn print_deep_indentation() {
    let f = deep_plist();
    let r = run_c("Print", &f);
    let expected = "\
Dict {
    a = Dict {
        b = Dict {
            c = Dict {
                d = deep
            }
        }
    }
}\n";
    assert_eq!(r.stdout, expected);
}

// ============================================================
// Print errors - go to stderr
// ============================================================

#[test]
fn print_nonexistent_entry_stderr() {
    let f = sample_plist();
    let r = run_c("Print :Nonexistent", &f);
    assert!(r.stdout.is_empty());
    assert_eq!(
        r.stderr.trim(),
        "Print: Entry, \":Nonexistent\", Does Not Exist"
    );
    assert_eq!(r.exit_code, 1);
}

#[test]
fn print_nonexistent_no_colon_error_format() {
    let f = sample_plist();
    let r = run_c("Print Nonexistent", &f);
    assert_eq!(
        r.stderr.trim(),
        "Print: Entry, \"Nonexistent\", Does Not Exist"
    );
    assert_eq!(r.exit_code, 1);
}

#[test]
fn print_deep_partial_nonexistent() {
    let f = deep_plist();
    let r = run_c("Print :a:b:x", &f);
    assert_eq!(
        r.stderr.trim(),
        "Print: Entry, \":a:b:x\", Does Not Exist"
    );
    assert_eq!(r.exit_code, 1);
}

#[test]
fn print_array_oob() {
    let f = root_array_plist();
    let r = run_c("Print :99", &f);
    assert_eq!(
        r.stderr.trim(),
        "Print: Entry, \":99\", Does Not Exist"
    );
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Set command
// ============================================================

#[test]
fn set_string_value() {
    let f = sample_plist();
    run_c("Set :Name NewName", &f);
    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "NewName\n");
}

#[test]
fn set_integer_value() {
    let f = sample_plist();
    run_c("Set :Version 99", &f);
    let r = run_c("Print :Version", &f);
    assert_eq!(r.stdout, "99\n");
}

#[test]
fn set_bool_value() {
    let f = sample_plist();
    run_c("Set :Enabled false", &f);
    let r = run_c("Print :Enabled", &f);
    assert_eq!(r.stdout, "false\n");
}

#[test]
fn set_with_quoted_value() {
    let f = sample_plist();
    run_c("Set :Name \"new name\"", &f);
    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "new name\n");
}

#[test]
fn set_with_spaces_in_value() {
    let f = sample_plist();
    run_c("Set :Name hello world", &f);
    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "hello world\n");
}

#[test]
fn set_nested_value() {
    let f = sample_plist();
    run_c("Set :Nested:Inner newval", &f);
    let r = run_c("Print :Nested:Inner", &f);
    assert_eq!(r.stdout, "newval\n");
}

#[test]
fn set_array_element() {
    let f = sample_plist();
    run_c("Set :Tags:0 replaced", &f);
    let r = run_c("Print :Tags:0", &f);
    assert_eq!(r.stdout, "replaced\n");
}

#[test]
fn set_nonexistent_error_stderr() {
    let f = sample_plist();
    let r = run_c("Set :Nonexistent foo", &f);
    assert!(r.stdout.is_empty());
    assert_eq!(
        r.stderr.trim(),
        "Set: Entry, \":Nonexistent\", Does Not Exist"
    );
    assert_eq!(r.exit_code, 1);
}

#[test]
fn set_container_error_stderr() {
    let f = sample_plist();
    let r = run_c("Set :Tags foo", &f);
    assert!(r.stdout.is_empty());
    assert_eq!(r.stderr.trim(), "Set: Cannot Perform Set On Containers");
    assert_eq!(r.exit_code, 1);
}

#[test]
fn set_dict_container_error_stderr() {
    let f = sample_plist();
    let r = run_c("Set :Nested foo", &f);
    assert!(r.stdout.is_empty());
    assert_eq!(r.stderr.trim(), "Set: Cannot Perform Set On Containers");
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Add command
// ============================================================

#[test]
fn add_string() {
    let f = empty_dict_plist();
    let r = run_c("Add :Key string hello", &f);
    assert!(r.stdout.is_empty());
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);
    let r2 = run_c("Print :Key", &f);
    assert_eq!(r2.stdout, "hello\n");
}

#[test]
fn add_integer() {
    let f = empty_dict_plist();
    run_c("Add :Num integer 42", &f);
    let r = run_c("Print :Num", &f);
    assert_eq!(r.stdout, "42\n");
}

#[test]
fn add_real() {
    let f = empty_dict_plist();
    run_c("Add :Pi real 3.14", &f);
    let r = run_c("Print :Pi", &f);
    assert_eq!(r.stdout, "3.140000\n");
}

#[test]
fn add_bool_true() {
    let f = empty_dict_plist();
    run_c("Add :Flag bool true", &f);
    let r = run_c("Print :Flag", &f);
    assert_eq!(r.stdout, "true\n");
}

#[test]
fn add_bool_false() {
    let f = empty_dict_plist();
    run_c("Add :Flag bool false", &f);
    let r = run_c("Print :Flag", &f);
    assert_eq!(r.stdout, "false\n");
}

#[test]
fn add_bool_yes() {
    let f = empty_dict_plist();
    run_c("Add :Flag bool YES", &f);
    let r = run_c("Print :Flag", &f);
    assert_eq!(r.stdout, "true\n");
}

#[test]
fn add_dict() {
    let f = empty_dict_plist();
    run_c("Add :Sub dict", &f);
    let r = run_c("Print :Sub", &f);
    assert_eq!(r.stdout, "Dict {\n}\n");
}

#[test]
fn add_array() {
    let f = empty_dict_plist();
    run_c("Add :Arr array", &f);
    let r = run_c("Print :Arr", &f);
    assert_eq!(r.stdout, "Array {\n}\n");
}

#[test]
fn add_data() {
    let f = empty_dict_plist();
    run_c("Add :D data AQID", &f);
    let r = run_c("Print :D", &f);
    assert_eq!(r.stdout.as_bytes(), b"AQID\n");
}

#[test]
fn add_empty_string() {
    let f = empty_dict_plist();
    run_c("Add :E string", &f);
    let r = run_c("Print :E", &f);
    assert_eq!(r.stdout, "\n");
}

#[test]
fn add_with_quoted_value() {
    let f = empty_dict_plist();
    run_c("Add :K string \"hello world\"", &f);
    let r = run_c("Print :K", &f);
    assert_eq!(r.stdout, "hello world\n");
}

#[test]
fn add_array_append() {
    let f = sample_plist();
    run_c("Add :Tags: string gamma", &f);
    let r = run_c("Print :Tags", &f);
    assert!(r.stdout.contains("gamma"));
    let r2 = run_c("Print :Tags:2", &f);
    assert_eq!(r2.stdout, "gamma\n");
}

#[test]
fn add_array_insert_at_zero() {
    let f = sample_plist();
    run_c("Add :Tags:0 string first", &f);
    let r = run_c("Print :Tags:0", &f);
    assert_eq!(r.stdout, "first\n");
    let r2 = run_c("Print :Tags:1", &f);
    assert_eq!(r2.stdout, "alpha\n");
}

#[test]
fn add_existing_entry_error_stderr() {
    let f = sample_plist();
    let r = run_c("Add :Name string foo", &f);
    assert!(r.stdout.is_empty());
    assert_eq!(r.stderr.trim(), "Add: \":Name\" Entry Already Exists");
    assert_eq!(r.exit_code, 1);
}

#[test]
fn add_unrecognized_type_stdout() {
    let f = sample_plist();
    let r = run_c("Add :Foo badtype bar", &f);
    assert_eq!(r.stdout.trim(), "Unrecognized Type: badtype");
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 1);
}

#[test]
fn add_auto_create_intermediate_dicts() {
    let f = empty_dict_plist();
    let r = run_c("Add :a:b:c string val", &f);
    assert_eq!(r.exit_code, 0);
    let r2 = run_c("Print :a:b:c", &f);
    assert_eq!(r2.stdout, "val\n");
}

#[test]
fn add_auto_create_deep() {
    let f = empty_dict_plist();
    run_c("Add :x:y:z:w string deep", &f);
    let r = run_c("Print :x:y:z:w", &f);
    assert_eq!(r.stdout, "deep\n");
}

#[test]
fn add_cant_add_to_scalar_parent_stderr() {
    let f = sample_plist();
    let r = run_c("Add :Name:sub string val", &f);
    assert!(r.stdout.is_empty());
    assert_eq!(
        r.stderr.trim(),
        "Add: Can't Add Entry, \":Name:sub\", to Parent"
    );
    assert_eq!(r.exit_code, 1);
}

#[test]
fn add_cant_add_to_array_string_element() {
    let f = root_array_plist();
    let r = run_c("Add :0:sub string val", &f);
    assert_eq!(
        r.stderr.trim(),
        "Add: Can't Add Entry, \":0:sub\", to Parent"
    );
    assert_eq!(r.exit_code, 1);
}

#[test]
fn add_root_array_append() {
    let f = root_array_plist();
    let r = run_c("Add : string appended", &f);
    assert_eq!(r.exit_code, 0);
    let r2 = run_c("Print :2", &f);
    assert_eq!(r2.stdout, "appended\n");
}

#[test]
fn add_root_array_insert() {
    let f = root_array_plist();
    run_c("Add :0 string inserted", &f);
    let r = run_c("Print :0", &f);
    assert_eq!(r.stdout, "inserted\n");
    let r2 = run_c("Print :1", &f);
    assert_eq!(r2.stdout, "first\n");
}

#[test]
fn add_empty_key_to_dict() {
    let f = empty_dict_plist();
    let r = run_c("Add : string val", &f);
    assert_eq!(r.exit_code, 0);
    let r2 = run_c("Print :", &f);
    // Prints the whole dict since : resolves to root
    assert!(r2.stdout.contains("= val"));
}

#[test]
fn add_to_nested_array_append() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<array>
		<string>inner0</string>
	</array>
</array>
</plist>"#,
    );
    run_c("Add :0: string added", &f);
    let r = run_c("Print :0:1", &f);
    assert_eq!(r.stdout, "added\n");
}

// ============================================================
// Copy command
// ============================================================

#[test]
fn copy_entry() {
    let f = sample_plist();
    run_c("Copy :Name :NameCopy", &f);
    let r = run_c("Print :NameCopy", &f);
    assert_eq!(r.stdout, "Test App\n");
}

#[test]
fn copy_to_existing_error_stderr() {
    let f = sample_plist();
    let r = run_c("Copy :Name :Version", &f);
    assert!(r.stdout.is_empty());
    assert_eq!(r.stderr.trim(), "Copy: \":Version\" Entry Already Exists");
    assert_eq!(r.exit_code, 1);
}

#[test]
fn copy_nonexistent_source_stderr() {
    let f = sample_plist();
    let r = run_c("Copy :Nonexistent :Foo", &f);
    assert!(r.stdout.is_empty());
    assert_eq!(
        r.stderr.trim(),
        "Copy: Entry, \":Nonexistent\", Does Not Exist"
    );
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Delete command
// ============================================================

#[test]
fn delete_key() {
    let f = sample_plist();
    run_c("Delete :Name", &f);
    let r = run_c("Print :Name", &f);
    assert_eq!(r.exit_code, 1);
}

#[test]
fn delete_array_element() {
    let f = sample_plist();
    run_c("Delete :Tags:0", &f);
    let r = run_c("Print :Tags:0", &f);
    assert_eq!(r.stdout, "beta\n");
}

#[test]
fn delete_entire_subtree() {
    let f = deep_plist();
    run_c("Delete :a:b", &f);
    let r = run_c("Print :a", &f);
    assert_eq!(r.stdout, "Dict {\n}\n");
}

#[test]
fn delete_nonexistent_stderr() {
    let f = sample_plist();
    let r = run_c("Delete :Nonexistent", &f);
    assert!(r.stdout.is_empty());
    assert_eq!(
        r.stderr.trim(),
        "Delete: Entry, \":Nonexistent\", Does Not Exist"
    );
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Clear command
// ============================================================

#[test]
fn clear_dict() {
    let f = sample_plist();
    let r = run_c("Clear dict", &f);
    assert_eq!(r.stdout, "Initializing Plist...\n");
    assert!(r.stderr.is_empty());
    let r2 = run_c("Print", &f);
    assert_eq!(r2.stdout, "Dict {\n}\n");
}

#[test]
fn clear_array() {
    let f = sample_plist();
    let r = run_c("Clear array", &f);
    assert_eq!(r.stdout, "Initializing Plist...\n");
    let r2 = run_c("Print", &f);
    assert_eq!(r2.stdout, "Array {\n}\n");
}

#[test]
fn clear_no_type_defaults_to_dict_with_warning() {
    let f = sample_plist();
    let r = run_c("Clear", &f);
    assert_eq!(r.stdout, "Unrecognized Type: \nInitializing Plist...\n");
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);
    let r2 = run_c("Print", &f);
    assert_eq!(r2.stdout, "Dict {\n}\n");
}

// ============================================================
// Merge command
// ============================================================

#[test]
fn merge_adds_keys() {
    let source = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>NewKey</key>
	<string>merged</string>
</dict>
</plist>"#,
    );
    let f = sample_plist();
    run_c(&format!("Merge {}", source.display()), &f);
    let r = run_c("Print :NewKey", &f);
    assert_eq!(r.stdout, "merged\n");
}

#[test]
fn merge_into_entry() {
    let source = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Extra</key>
	<string>extra</string>
</dict>
</plist>"#,
    );
    let f = sample_plist();
    run_c(&format!("Merge {} :Nested", source.display()), &f);
    let r = run_c("Print :Nested:Extra", &f);
    assert_eq!(r.stdout, "extra\n");
    // Original key still present
    let r2 = run_c("Print :Nested:Inner", &f);
    assert_eq!(r2.stdout, "value\n");
}

#[test]
fn merge_nonexistent_file_stderr() {
    let f = sample_plist();
    let r = run_c("Merge /tmp/no_such_file_merge_test.plist", &f);
    assert!(r.stdout.is_empty());
    assert!(r.stderr.contains("Error Opening File:"));
    assert!(r.stderr.contains("Merge: Error Reading File:"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Import command
// ============================================================

#[test]
fn import_creates_data_entry() {
    let data_file = std::env::temp_dir().join("re_pb_import_test.txt");
    fs::write(&data_file, "hello world").unwrap();

    let f = sample_plist();
    run_c(
        &format!("Import :Imported {}", data_file.display()),
        &f,
    );
    let r = run_c("Print :Imported", &f);
    assert_eq!(r.stdout, "hello world\n");

    fs::remove_file(&data_file).ok();
}

#[test]
fn import_overwrites_existing() {
    let data_file = std::env::temp_dir().join("re_pb_import_overwrite.txt");
    fs::write(&data_file, "new data").unwrap();

    let f = sample_plist();
    run_c(
        &format!("Import :Name {}", data_file.display()),
        &f,
    );
    // Name was a string, now should be data
    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "new data\n");

    fs::remove_file(&data_file).ok();
}

#[test]
fn import_nonexistent_file_stderr() {
    let f = sample_plist();
    let r = run_c("Import :Foo /tmp/no_such_file_import_test.txt", &f);
    assert!(r.stdout.is_empty());
    assert!(r.stderr.contains("Error Opening File:"));
    assert!(r.stderr.contains("No such file or directory"));
    assert!(r.stderr.contains("Import: Error Reading File:"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Help command - stdout
// ============================================================

#[test]
fn help_command_stdout() {
    let f = sample_plist();
    let r = run_c("Help", &f);
    assert!(r.stdout.contains("Command Format:"));
    assert!(r.stdout.contains("Entry Format:"));
    assert!(r.stdout.contains("Types:"));
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);
}

// ============================================================
// Unrecognized command - stdout
// ============================================================

#[test]
fn unrecognized_command_stdout() {
    let f = sample_plist();
    let r = run_c("FooBar", &f);
    assert_eq!(r.stdout.trim(), "Unrecognized Command");
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 1);
}

#[test]
fn empty_command_stdout() {
    let f = sample_plist();
    let r = run_c("", &f);
    assert_eq!(r.stdout.trim(), "Unrecognized Command");
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Save / Revert / Exit in -c mode
// ============================================================

#[test]
fn save_message_stdout() {
    let f = sample_plist();
    let r = run_c("Save", &f);
    assert_eq!(r.stdout, "Saving...\n");
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);
}

#[test]
fn revert_message_stdout() {
    let f = sample_plist();
    let r = run_c("Revert", &f);
    assert_eq!(r.stdout, "Reverting to last saved state...\n");
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);
}

#[test]
fn exit_does_not_prevent_save_in_c_mode() {
    let f = sample_plist();
    run_multi_c(&["Set :Name ExitTest", "Exit"], &f);
    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "ExitTest\n");
}

#[test]
fn revert_undoes_changes() {
    let f = sample_plist();
    let r = run_multi_c(&["Set :Name Changed", "Revert", "Print :Name"], &f);
    assert!(r.stdout.contains("Test App"));
}

// ============================================================
// Multiple -c commands
// ============================================================

#[test]
fn multiple_c_all_succeed() {
    let f = sample_plist();
    let r = run_multi_c(&["Print :Name", "Print :Version"], &f);
    assert_eq!(r.stdout, "Test App\n42\n");
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);
}

#[test]
fn multiple_c_with_failure_returns_exit_1() {
    let f = sample_plist();
    let r = run_multi_c(
        &["Print :Name", "Print :Nonexistent", "Print :Version"],
        &f,
    );
    // stdout has the successful prints
    assert!(r.stdout.contains("Test App"));
    assert!(r.stdout.contains("42"));
    // stderr has the error
    assert!(r.stderr.contains("Print: Entry, \":Nonexistent\", Does Not Exist"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn multiple_c_set_overwrite() {
    let f = sample_plist();
    run_multi_c(
        &["Set :Name first", "Set :Name second", "Set :Name third"],
        &f,
    );
    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "third\n");
}

#[test]
fn delete_then_add_same_key() {
    let f = sample_plist();
    run_multi_c(&["Delete :Name", "Add :Name integer 42"], &f);
    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "42\n");
}

// ============================================================
// XML output mode (-x)
// ============================================================

#[test]
fn xml_output_string() {
    let f = sample_plist();
    let r = run(&["-x", "-c", "Print :Name", f.to_str().unwrap()]);
    assert!(r.stdout.contains("<string>Test App</string>"));
    assert!(r.stdout.starts_with("<?xml version="));
    assert!(r.stdout.contains("<!DOCTYPE plist"));
    assert!(r.stdout.ends_with("</plist>\n"));
}

#[test]
fn xml_output_integer() {
    let f = sample_plist();
    let r = run(&["-x", "-c", "Print :Version", f.to_str().unwrap()]);
    assert!(r.stdout.contains("<integer>42</integer>"));
}

#[test]
fn xml_output_bool() {
    let f = sample_plist();
    let r = run(&["-x", "-c", "Print :Enabled", f.to_str().unwrap()]);
    assert!(r.stdout.contains("<true/>"));
}

#[test]
fn xml_output_array() {
    let f = sample_plist();
    let r = run(&["-x", "-c", "Print :Tags", f.to_str().unwrap()]);
    assert!(r.stdout.contains("<array>"));
    assert!(r.stdout.contains("<string>alpha</string>"));
    assert!(r.stdout.contains("<string>beta</string>"));
}

#[test]
fn xml_output_dict() {
    let f = sample_plist();
    let r = run(&["-x", "-c", "Print :Nested", f.to_str().unwrap()]);
    assert!(r.stdout.contains("<dict>"));
    assert!(r.stdout.contains("<key>Inner</key>"));
    assert!(r.stdout.contains("<string>value</string>"));
}

// ============================================================
// New file creation
// ============================================================

#[test]
fn new_file_creation_message_stdout() {
    let path = std::env::temp_dir().join("re_pb_new_file_test.plist");
    let _ = fs::remove_file(&path);
    let r = run_c("Add :Key string val", &path);
    assert!(r.stdout.contains("File Doesn't Exist, Will Create:"));
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);

    let r2 = run_c("Print :Key", &path);
    assert_eq!(r2.stdout, "val\n");
    fs::remove_file(&path).ok();
}

// ============================================================
// Case insensitivity
// ============================================================

#[test]
fn command_case_insensitive() {
    let f = sample_plist();
    let r1 = run_c("print :Name", &f);
    assert_eq!(r1.stdout, "Test App\n");
    let r2 = run_c("PRINT :Name", &f);
    assert_eq!(r2.stdout, "Test App\n");
    let r3 = run_c("Print :Name", &f);
    assert_eq!(r3.stdout, "Test App\n");
}

#[test]
fn type_case_insensitive() {
    let f = empty_dict_plist();
    run_c("Add :K1 String hello", &f);
    let r1 = run_c("Print :K1", &f);
    assert_eq!(r1.stdout, "hello\n");

    run_c("Add :K2 INTEGER 5", &f);
    let r2 = run_c("Print :K2", &f);
    assert_eq!(r2.stdout, "5\n");
}

// ============================================================
// Quote-aware tokenizer
// ============================================================

#[test]
fn print_only_uses_first_token() {
    let f = special_keys_plist();
    let r = run_c("Print :key with spaces", &f);
    // "key" is not a key in the plist, so error
    assert_eq!(r.exit_code, 1);
    assert!(r.stderr.contains("Does Not Exist"));
}

#[test]
fn quoted_entry_groups_words() {
    let f = special_keys_plist();
    let r = run_c("Print \":key with spaces\"", &f);
    assert_eq!(r.stdout, "spaced\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn embedded_quotes_stripped() {
    // key"quotes" in input becomes keyquotes after quote stripping
    let f = special_keys_plist();
    let r = run_c("Print :key\"quotes\"", &f);
    // "keyquotes" doesn't exist
    assert_eq!(r.exit_code, 1);
}

#[test]
fn emoji_key() {
    let f = special_keys_plist();
    let r = run_c("Print :emoji🎉", &f);
    assert_eq!(r.stdout, "party\n");
}

#[test]
fn special_chars_key() {
    let f = special_keys_plist();
    let r = run_c("Print :key&special", &f);
    assert_eq!(r.stdout, "special\n");
}

// ============================================================
// Root array operations
// ============================================================

#[test]
fn root_array_print() {
    let f = root_array_plist();
    let r = run_c("Print", &f);
    assert_eq!(r.stdout, "Array {\n    first\n    second\n}\n");
}

#[test]
fn root_array_print_element() {
    let f = root_array_plist();
    let r = run_c("Print :0", &f);
    assert_eq!(r.stdout, "first\n");
    let r2 = run_c("Print :1", &f);
    assert_eq!(r2.stdout, "second\n");
}

#[test]
fn root_array_set_element() {
    let f = root_array_plist();
    run_c("Set :0 replaced", &f);
    let r = run_c("Print :0", &f);
    assert_eq!(r.stdout, "replaced\n");
}

#[test]
fn root_array_delete_element() {
    let f = root_array_plist();
    run_c("Delete :0", &f);
    let r = run_c("Print :0", &f);
    assert_eq!(r.stdout, "second\n");
}

// ============================================================
// Empty plist operations
// ============================================================

#[test]
fn empty_dict_print() {
    let f = empty_dict_plist();
    let r = run_c("Print", &f);
    assert_eq!(r.stdout, "Dict {\n}\n");
}

#[test]
fn empty_array_print() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array/>
</plist>"#,
    );
    let r = run_c("Print", &f);
    assert_eq!(r.stdout, "Array {\n}\n");
}

#[test]
fn empty_array_append() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array/>
</plist>"#,
    );
    run_c("Add : string first", &f);
    let r = run_c("Print :0", &f);
    assert_eq!(r.stdout, "first\n");
}

// ============================================================
// Multiline string
// ============================================================

#[test]
fn multiline_string_print() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Multi</key>
	<string>line1
line2
line3</string>
</dict>
</plist>"#,
    );
    let r = run_c("Print :Multi", &f);
    assert_eq!(r.stdout, "line1\nline2\nline3\n");
}

// ============================================================
// Big integers and negative numbers
// ============================================================

#[test]
fn big_integer() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Big</key>
	<integer>9999999999999999</integer>
	<key>Neg</key>
	<integer>-42</integer>
	<key>Zero</key>
	<integer>0</integer>
</dict>
</plist>"#,
    );
    let r = run_c("Print :Big", &f);
    assert_eq!(r.stdout, "9999999999999999\n");
    let r2 = run_c("Print :Neg", &f);
    assert_eq!(r2.stdout, "-42\n");
    let r3 = run_c("Print :Zero", &f);
    assert_eq!(r3.stdout, "0\n");
}

// ============================================================
// Real number edge cases
// ============================================================

#[test]
fn negative_real() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>NegR</key>
	<real>-3.14</real>
	<key>ZeroR</key>
	<real>0.0</real>
	<key>TinyR</key>
	<real>0.000001</real>
</dict>
</plist>"#,
    );
    let r = run_c("Print :NegR", &f);
    assert_eq!(r.stdout, "-3.140000\n");
    let r2 = run_c("Print :ZeroR", &f);
    assert_eq!(r2.stdout, "0.000000\n");
    let r3 = run_c("Print :TinyR", &f);
    assert_eq!(r3.stdout, "0.000001\n");
}

// ============================================================
// Boolean edge cases
// ============================================================

#[test]
fn bool_set_yes_and_false() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>B</key>
	<false/>
</dict>
</plist>"#,
    );
    run_c("Set :B YES", &f);
    let r = run_c("Print :B", &f);
    assert_eq!(r.stdout, "true\n");

    run_c("Set :B false", &f);
    let r2 = run_c("Print :B", &f);
    assert_eq!(r2.stdout, "false\n");
}

// ============================================================
// Mixed array with all types
// ============================================================

#[test]
fn mixed_array_elements() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<string>text</string>
	<integer>42</integer>
	<real>3.14</real>
	<true/>
	<false/>
	<array>
		<string>nested</string>
	</array>
	<dict>
		<key>k</key>
		<string>v</string>
	</dict>
</array>
</plist>"#,
    );
    let r0 = run_c("Print :0", &f);
    assert_eq!(r0.stdout, "text\n");
    let r1 = run_c("Print :1", &f);
    assert_eq!(r1.stdout, "42\n");
    let r2 = run_c("Print :2", &f);
    assert_eq!(r2.stdout, "3.140000\n");
    let r3 = run_c("Print :3", &f);
    assert_eq!(r3.stdout, "true\n");
    let r4 = run_c("Print :4", &f);
    assert_eq!(r4.stdout, "false\n");
    let r5 = run_c("Print :5:0", &f);
    assert_eq!(r5.stdout, "nested\n");
    let r6 = run_c("Print :6:k", &f);
    assert_eq!(r6.stdout, "v\n");
}

// ============================================================
// -c mode auto-saves
// ============================================================

#[test]
fn c_mode_auto_saves() {
    let f = sample_plist();
    run_c("Set :Name Saved", &f);
    // Read back from file without modifying
    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "Saved\n");
}

// ============================================================
// Binary plist support
// ============================================================

#[test]
fn reads_binary_plist() {
    let f = sample_plist();
    let binary_path = std::env::temp_dir().join("re_pb_binary_test.plist");

    let val = re_plistbuddy::value::Value::from_file(&f).unwrap();
    val.to_file_binary(&binary_path).unwrap();

    let r = run_c("Print :Name", &binary_path);
    assert_eq!(r.stdout, "Test App\n");

    fs::remove_file(&binary_path).ok();
}

// ============================================================
// Root scalar plists
// ============================================================

fn root_string_plist() -> PathBuf {
    temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<string>just a string</string>
</plist>"#,
    )
}

fn root_integer_plist() -> PathBuf {
    temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<integer>42</integer>
</plist>"#,
    )
}

fn root_bool_plist() -> PathBuf {
    temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<true/>
</plist>"#,
    )
}

#[test]
fn root_string_print() {
    let f = root_string_plist();
    let r = run_c("Print", &f);
    assert_eq!(r.stdout, "just a string\n");
}

#[test]
fn root_string_set_via_colon() {
    let f = root_string_plist();
    run_c("Set : newvalue", &f);
    let r = run_c("Print", &f);
    assert_eq!(r.stdout, "newvalue\n");
}

#[test]
fn root_integer_set_via_colon() {
    let f = root_integer_plist();
    run_c("Set : 99", &f);
    let r = run_c("Print", &f);
    assert_eq!(r.stdout, "99\n");
}

#[test]
fn root_bool_set_via_colon() {
    let f = root_bool_plist();
    run_c("Set : false", &f);
    let r = run_c("Print", &f);
    assert_eq!(r.stdout, "false\n");
}

#[test]
fn root_scalar_add_fails() {
    let f = root_string_plist();
    let r = run_c("Add :key string val", &f);
    assert_eq!(
        r.stderr.trim(),
        "Add: Can't Add Entry, \":key\", to Parent"
    );
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Leading whitespace in command
// ============================================================

#[test]
fn leading_spaces_unrecognized() {
    let f = sample_plist();
    let r = run_c("  Print :Name", &f);
    assert_eq!(r.stdout.trim(), "Unrecognized Command");
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Type coercion errors
// ============================================================

#[test]
fn set_integer_non_numeric_preserves_value() {
    let f = sample_plist();
    let r = run_c("Set :Version abc", &f);
    assert_eq!(r.stdout.trim(), "Unrecognized Integer Format");
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);
    let r2 = run_c("Print :Version", &f);
    assert_eq!(r2.stdout, "42\n");
}

#[test]
fn set_real_non_numeric_preserves_value() {
    let f = sample_plist();
    let r = run_c("Set :Rating abc", &f);
    assert_eq!(r.stdout.trim(), "Unrecognized Real Format");
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);
    let r2 = run_c("Print :Rating", &f);
    assert_eq!(r2.stdout, "3.140000\n");
}

// ============================================================
// No-args commands
// ============================================================

#[test]
fn set_no_args_container_error() {
    let f = sample_plist();
    let r = run_c("Set", &f);
    assert_eq!(r.stderr.trim(), "Set: Cannot Perform Set On Containers");
    assert_eq!(r.exit_code, 1);
}

#[test]
fn add_no_args_unrecognized_type() {
    let f = sample_plist();
    let r = run_c("Add", &f);
    assert_eq!(r.stdout.trim(), "Unrecognized Type:");
    assert_eq!(r.exit_code, 1);
}

#[test]
fn copy_no_args_already_exists() {
    let f = sample_plist();
    let r = run_c("Copy", &f);
    assert_eq!(r.stderr.trim(), "Copy: \"\" Entry Already Exists");
    assert_eq!(r.exit_code, 1);
}

#[test]
fn delete_no_args_working_container_message() {
    let f = sample_plist();
    let r = run_c("Delete", &f);
    assert_eq!(
        r.stdout.trim(),
        "Working Container has become Invalid.  Setting to :"
    );
    assert_eq!(r.exit_code, 0);
}

#[test]
fn merge_no_args_file_error() {
    let f = sample_plist();
    let r = run_c("Merge", &f);
    assert!(r.stderr.contains("Error Opening File:"));
    assert!(r.stderr.contains("Merge: Error Reading File:"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn import_no_args_container_error() {
    let f = sample_plist();
    let r = run_c("Import", &f);
    assert_eq!(
        r.stderr.trim(),
        "Import: Specified Entry Must Not Be a Container"
    );
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Import to container
// ============================================================

#[test]
fn import_to_dict_entry_fails() {
    let data_file = std::env::temp_dir().join("re_pb_import_container_test.txt");
    fs::write(&data_file, "data").unwrap();
    let f = sample_plist();
    let r = run_c(
        &format!("Import :Nested {}", data_file.display()),
        &f,
    );
    assert_eq!(
        r.stderr.trim(),
        "Import: Specified Entry Must Not Be a Container"
    );
    assert_eq!(r.exit_code, 1);
    fs::remove_file(&data_file).ok();
}

#[test]
fn import_to_array_entry_fails() {
    let data_file = std::env::temp_dir().join("re_pb_import_arr_test.txt");
    fs::write(&data_file, "data").unwrap();
    let f = sample_plist();
    let r = run_c(
        &format!("Import :Tags {}", data_file.display()),
        &f,
    );
    assert_eq!(
        r.stderr.trim(),
        "Import: Specified Entry Must Not Be a Container"
    );
    assert_eq!(r.exit_code, 1);
    fs::remove_file(&data_file).ok();
}

// ============================================================
// Clear all types
// ============================================================

#[test]
fn clear_string_type() {
    let f = sample_plist();
    let r = run_c("Clear string", &f);
    assert_eq!(r.stdout, "Initializing Plist...\n");
    let r2 = run_c("Print", &f);
    assert_eq!(r2.stdout, "\n");
}

#[test]
fn clear_integer_type() {
    let f = sample_plist();
    run_c("Clear integer", &f);
    let r = run_c("Print", &f);
    assert_eq!(r.stdout, "0\n");
}

#[test]
fn clear_bool_type() {
    let f = sample_plist();
    run_c("Clear bool", &f);
    let r = run_c("Print", &f);
    assert_eq!(r.stdout, "false\n");
}

// ============================================================
// Add past end of array
// ============================================================

#[test]
fn add_past_end_appends() {
    let f = sample_plist();
    let r = run_c("Add :Tags:99 string x", &f);
    assert_eq!(r.exit_code, 0);
    let r2 = run_c("Print :Tags:2", &f);
    assert_eq!(r2.stdout, "x\n");
}

// ============================================================
// Copy with auto-create intermediate dicts
// ============================================================

#[test]
fn copy_creates_intermediate_dicts() {
    let f = sample_plist();
    let r = run_c("Copy :Name :New:Deep:Path", &f);
    assert_eq!(r.exit_code, 0);
    let r2 = run_c("Print :New:Deep:Path", &f);
    assert_eq!(r2.stdout, "Test App\n");
}

#[test]
fn copy_with_only_source_is_noop() {
    let f = sample_plist();
    let r = run_c("Copy :Name", &f);
    assert_eq!(r.exit_code, 0);
    assert!(r.stderr.is_empty());
}

// ============================================================
// Merge does not overwrite existing keys
// ============================================================

#[test]
fn merge_skips_existing_keys() {
    let source = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Name</key>
	<string>from merge</string>
</dict>
</plist>"#,
    );
    let f = sample_plist();
    run_c(&format!("Merge {}", source.display()), &f);
    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "Test App\n");
}

// ============================================================
// Import with only entry (no file path)
// ============================================================

#[test]
fn import_only_entry_file_error() {
    let f = sample_plist();
    let r = run_c("Import :Foo", &f);
    assert!(r.stderr.contains("Error Opening File:"));
    assert!(r.stderr.contains("Import: Error Reading File:"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Set string to numeric preserves type
// ============================================================

#[test]
fn set_string_to_numeric_stays_string() {
    let f = sample_plist();
    run_c("Set :Name 42", &f);
    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "42\n");
    // Verify it's still a string via XML output
    let r2 = run(&["-x", "-c", "Print :Name", f.to_str().unwrap()]);
    assert!(r2.stdout.contains("<string>42</string>"));
}

// ============================================================
// Combined flags are NOT supported
// ============================================================

#[test]
fn combined_flags_treated_as_filename() {
    // -xc is not -x + -c, it's a file path
    let r = run(&["-xc", "Print :Name", "/tmp/test_pb.plist"]);
    assert!(r.stdout.contains("Invalid Arguments"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn extra_positional_args_rejected() {
    let f = sample_plist();
    let r = run(&[f.to_str().unwrap(), "extra"]);
    assert!(r.stdout.contains("Invalid Arguments"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Symlink handling with -l
// ============================================================

#[test]
fn dash_l_blocks_symlinks() {
    let target = sample_plist();
    let link = std::env::temp_dir().join("re_pb_symlink_test.plist");
    let _ = fs::remove_file(&link);
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let r = run(&["-l", "-c", "Print :Name", link.to_str().unwrap()]);
    assert!(r.stderr.contains("Too many levels of symbolic links"));
    assert_eq!(r.exit_code, 1);

    fs::remove_file(&link).ok();
}

#[test]
fn dash_l_allows_regular_files() {
    let f = sample_plist();
    let r = run(&["-l", "-c", "Print :Name", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "Test App\n");
    assert_eq!(r.exit_code, 0);
}

// ============================================================
// Add date with no value
// ============================================================

#[test]
fn add_date_no_value_warns_but_succeeds() {
    let f = sample_plist();
    let r = run_c("Add :D date", &f);
    assert_eq!(r.stdout.trim(), "Unrecognized Date Format");
    assert_eq!(r.exit_code, 0);
    // Entry should NOT be created
    let r2 = run_c("Print :D", &f);
    assert_eq!(r2.exit_code, 1);
}

// ============================================================
// Leading whitespace in commands
// ============================================================

#[test]
fn leading_whitespace_rejected() {
    let f = sample_plist();
    let r = run_c("  Print :Name", &f);
    assert_eq!(r.stdout.trim(), "Unrecognized Command");
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Array of dicts
// ============================================================

#[test]
fn array_of_dicts_access() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<dict>
		<key>name</key>
		<string>Alice</string>
	</dict>
	<dict>
		<key>name</key>
		<string>Bob</string>
	</dict>
</array>
</plist>"#,
    );
    let r = run_c("Print :0:name", &f);
    assert_eq!(r.stdout, "Alice\n");
    let r2 = run_c("Print :1:name", &f);
    assert_eq!(r2.stdout, "Bob\n");
}

// ============================================================
// Unicode values
// ============================================================

#[test]
fn unicode_emoji_value() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>emoji</key>
	<string>🎉🎊🎈</string>
</dict>
</plist>"#,
    );
    let r = run_c("Print :emoji", &f);
    assert_eq!(r.stdout, "🎉🎊🎈\n");
}

#[test]
fn unicode_cjk_value() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>cjk</key>
	<string>你好世界</string>
</dict>
</plist>"#,
    );
    let r = run_c("Print :cjk", &f);
    assert_eq!(r.stdout, "你好世界\n");
}

// ============================================================
// XML special characters in values
// ============================================================

#[test]
fn xml_special_chars_in_value() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>html</key>
	<string>&lt;b&gt;bold&lt;/b&gt;</string>
	<key>amp</key>
	<string>a &amp; b</string>
</dict>
</plist>"#,
    );
    let r = run_c("Print :html", &f);
    assert_eq!(r.stdout, "<b>bold</b>\n");
    let r2 = run_c("Print :amp", &f);
    assert_eq!(r2.stdout, "a & b\n");
}

// ============================================================
// Integer boundaries
// ============================================================

#[test]
fn integer_max_min() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>max</key>
	<integer>9223372036854775807</integer>
	<key>min</key>
	<integer>-9223372036854775808</integer>
</dict>
</plist>"#,
    );
    let r = run_c("Print :max", &f);
    assert_eq!(r.stdout, "9223372036854775807\n");
    let r2 = run_c("Print :min", &f);
    assert_eq!(r2.stdout, "-9223372036854775808\n");
}

// ============================================================
// Deep nesting (10 levels)
// ============================================================

#[test]
fn deep_nesting_10_levels() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>a</key><dict>
	<key>b</key><dict>
	<key>c</key><dict>
	<key>d</key><dict>
	<key>e</key><dict>
	<key>f</key><dict>
	<key>g</key><dict>
	<key>h</key><dict>
	<key>i</key><dict>
	<key>j</key><string>deep10</string>
	</dict></dict></dict></dict></dict></dict></dict></dict></dict>
</dict>
</plist>"#,
    );
    let r = run_c("Print :a:b:c:d:e:f:g:h:i:j", &f);
    assert_eq!(r.stdout, "deep10\n");
}

// ============================================================
// Set value with equals sign
// ============================================================

#[test]
fn set_value_with_equals() {
    let f = sample_plist();
    run_c("Set :Name key=value", &f);
    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "key=value\n");
}

// ============================================================
// Type coercion: integer and real format errors (stdout, not stderr)
// ============================================================

#[test]
fn set_integer_non_numeric_stdout() {
    let f = sample_plist();
    let r = run_c("Set :Version abc", &f);
    assert_eq!(r.stdout.trim(), "Unrecognized Integer Format");
    assert!(r.stderr.is_empty());
}

#[test]
fn set_real_non_numeric_stdout() {
    let f = sample_plist();
    let r = run_c("Set :Rating abc", &f);
    assert_eq!(r.stdout.trim(), "Unrecognized Real Format");
    assert!(r.stderr.is_empty());
}

// ============================================================
// Corrupted / malformed plist files
// ============================================================

#[test]
fn empty_file_error() {
    let f = temp_plist("");
    // Overwrite with actually empty content (temp_plist writes the string)
    fs::write(&f, b"").unwrap();
    let r = run_c("Print", &f);
    assert!(r.stdout.contains("Error Reading File:"));
    assert!(!r.stderr.is_empty()); // CF error detail goes to stderr
    assert_eq!(r.exit_code, 1);
}

#[test]
fn random_garbage_error() {
    let f = std::env::temp_dir().join("re_pb_garbage.plist");
    fs::write(&f, b"\xff\xfe\xfd\xfc\xfb\xfa\x00\x01\x02\x03").unwrap();
    let r = run_c("Print", &f);
    assert!(r.stdout.contains("Error Reading File:"));
    assert_eq!(r.exit_code, 1);
    fs::remove_file(&f).ok();
}

#[test]
fn binary_plist_wrong_magic() {
    // Valid binary plist starts with "bplist00"
    let f = sample_plist();
    // Write via CF to get binary, then corrupt magic
    let val = re_plistbuddy::value::Value::from_file(&f).unwrap();
    let bin_path = std::env::temp_dir().join("re_pb_bad_magic.plist");
    val.to_file_binary(&bin_path).unwrap();

    let mut data = fs::read(&bin_path).unwrap();
    data[0] = b'X';
    data[1] = b'X';
    fs::write(&bin_path, &data).unwrap();

    let r = run_c("Print", &bin_path);
    assert!(r.stdout.contains("Error Reading File:"));
    assert_eq!(r.exit_code, 1);
    fs::remove_file(&bin_path).ok();
}

#[test]
fn truncated_binary_plist() {
    let f = sample_plist();
    let val = re_plistbuddy::value::Value::from_file(&f).unwrap();
    let bin_path = std::env::temp_dir().join("re_pb_truncated_bin.plist");
    val.to_file_binary(&bin_path).unwrap();

    // Keep only first 10 bytes
    let data = fs::read(&bin_path).unwrap();
    fs::write(&bin_path, &data[..10]).unwrap();

    let r = run_c("Print", &bin_path);
    assert!(r.stdout.contains("Error Reading File:"));
    assert_eq!(r.exit_code, 1);
    fs::remove_file(&bin_path).ok();
}

#[test]
fn truncated_xml_plist() {
    let f = std::env::temp_dir().join("re_pb_truncated_xml.plist");
    fs::write(
        &f,
        b"<?xml version=\"1.0\"?>\n<!DOCTYPE plist",
    )
    .unwrap();
    let r = run_c("Print", &f);
    assert!(r.stdout.contains("Error Reading File:"));
    assert_eq!(r.exit_code, 1);
    fs::remove_file(&f).ok();
}

#[test]
fn valid_xml_but_not_plist() {
    let f = std::env::temp_dir().join("re_pb_not_plist.plist");
    fs::write(&f, b"<?xml version=\"1.0\"?><html><body>hi</body></html>").unwrap();
    let r = run_c("Print", &f);
    assert!(r.stdout.contains("Error Reading File:"));
    assert_eq!(r.exit_code, 1);
    fs::remove_file(&f).ok();
}

#[test]
fn plain_text_file() {
    let f = std::env::temp_dir().join("re_pb_plain_text.plist");
    fs::write(&f, b"hello world\n").unwrap();
    let r = run_c("Print", &f);
    assert!(r.stdout.contains("Error Reading File:"));
    assert_eq!(r.exit_code, 1);
    fs::remove_file(&f).ok();
}

#[test]
fn binary_plist_corrupted_middle() {
    let f = sample_plist();
    let val = re_plistbuddy::value::Value::from_file(&f).unwrap();
    let bin_path = std::env::temp_dir().join("re_pb_bad_middle.plist");
    val.to_file_binary(&bin_path).unwrap();

    let mut data = fs::read(&bin_path).unwrap();
    if data.len() > 24 {
        data[20] = 0xff;
        data[21] = 0xff;
        data[22] = 0xff;
        data[23] = 0xff;
    }
    fs::write(&bin_path, &data).unwrap();

    // May succeed or fail depending on what bytes were corrupted
    let r = run_c("Print", &bin_path);
    // Just verify it doesn't crash (exit code may be 0 or 1)
    assert!(r.exit_code == 0 || r.exit_code == 1);
    fs::remove_file(&bin_path).ok();
}

#[test]
fn error_reading_file_goes_to_stdout() {
    let f = std::env::temp_dir().join("re_pb_err_stream.plist");
    fs::write(&f, b"not a plist").unwrap();
    let r = run_c("Print", &f);
    // "Error Reading File:" must be on stdout, not stderr
    assert!(r.stdout.contains("Error Reading File:"));
    // The CF error detail is on stderr
    assert!(!r.stderr.is_empty());
    fs::remove_file(&f).ok();
}

#[test]
fn corrupted_file_does_not_crash() {
    // Test various byte patterns that could cause issues
    for pattern in &[
        b"\x00\x00\x00\x00\x00\x00\x00\x00".as_slice(),
        b"bplist00".as_slice(),
        b"bplist00\x00\x00\x00\x00\x00\x00\x00\x00".as_slice(),
        b"\xff\xff\xff\xff\xff\xff\xff\xff".as_slice(),
    ] {
        let f = std::env::temp_dir().join("re_pb_crash_test.plist");
        fs::write(&f, pattern).unwrap();
        let r = run_c("Print", &f);
        // Must not crash - just error gracefully
        assert!(r.exit_code == 0 || r.exit_code == 1,
            "crashed on pattern: {:?}", pattern);
        fs::remove_file(&f).ok();
    }
}

// ============================================================
// Read-only commands don't write the file
// ============================================================

#[test]
fn print_does_not_write_readonly_file() {
    let f = sample_plist();
    let mut perms = fs::metadata(&f).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&f, perms).unwrap();

    let r = run_c("Print :Name", &f);
    assert_eq!(r.stdout, "Test App\n");
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&f).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(&f, perms).unwrap();
}

#[test]
fn multiple_prints_dont_write_readonly_file() {
    let f = sample_plist();
    let mut perms = fs::metadata(&f).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&f, perms).unwrap();

    let r = run_multi_c(&["Print :Name", "Print :Version"], &f);
    assert!(r.stdout.contains("Test App"));
    assert!(r.stdout.contains("42"));
    assert!(r.stderr.is_empty());
    assert_eq!(r.exit_code, 0);

    let mut perms = fs::metadata(&f).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(&f, perms).unwrap();
}

#[test]
fn help_does_not_write_readonly_file() {
    let f = sample_plist();
    let mut perms = fs::metadata(&f).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&f, perms).unwrap();

    let r = run_c("Help", &f);
    assert!(r.stdout.contains("Command Format:"));
    assert_eq!(r.exit_code, 0);

    let mut perms = fs::metadata(&f).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(&f, perms).unwrap();
}

#[test]
fn set_on_readonly_file_fails_on_write() {
    let f = sample_plist();
    let mut perms = fs::metadata(&f).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&f, perms).unwrap();

    let r = run_c("Set :Name changed", &f);
    assert_eq!(r.exit_code, 1);

    let mut perms = fs::metadata(&f).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(&f, perms).unwrap();
}
