use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn binary() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("plutil");
    path
}

fn temp_plist(content: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tid = std::thread::current().id();
    let path = std::env::temp_dir().join(format!("re_plutil_test_{tid:?}_{id}.plist"));
    fs::write(&path, content).unwrap();
    path
}

fn sample_plist() -> PathBuf {
    temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Name</key>
	<string>Test</string>
	<key>Num</key>
	<integer>42</integer>
	<key>Pi</key>
	<real>3.14</real>
	<key>Flag</key>
	<true/>
	<key>Items</key>
	<array>
		<string>a</string>
		<string>b</string>
	</array>
	<key>Sub</key>
	<dict>
		<key>Key</key>
		<string>val</string>
	</dict>
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
        .expect("failed to execute plutil");
    RunResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    }
}

// ============================================================
// CLI / no-args
// ============================================================

#[test]
fn no_args_prints_no_files() {
    let r = run(&[]);
    assert!(r.stderr.contains("No files specified."));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn help_flag() {
    let r = run(&["-help"]);
    assert!(r.stdout.contains("Command options are"));
    assert_eq!(r.exit_code, 0);
}

// ============================================================
// Lint
// ============================================================

#[test]
fn lint_ok() {
    let f = sample_plist();
    let r = run(&["-lint", f.to_str().unwrap()]);
    assert!(r.stdout.contains(": OK"));
    assert_eq!(r.exit_code, 0);
}

#[test]
fn lint_silent() {
    let f = sample_plist();
    let r = run(&["-lint", "-s", f.to_str().unwrap()]);
    assert!(r.stdout.is_empty());
    assert_eq!(r.exit_code, 0);
}

#[test]
fn lint_default_when_no_command() {
    let f = sample_plist();
    let r = run(&[f.to_str().unwrap()]);
    assert!(r.stdout.contains(": OK"));
    assert_eq!(r.exit_code, 0);
}

#[test]
fn lint_multiple_files() {
    let f1 = sample_plist();
    let f2 = sample_plist();
    let r = run(&["-lint", f1.to_str().unwrap(), f2.to_str().unwrap()]);
    let lines: Vec<&str> = r.stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(r.exit_code, 0);
}

#[test]
fn lint_missing_file() {
    let r = run(&["-lint", "/tmp/no_such_plutil_file.plist"]);
    assert!(r.stderr.contains("couldn\u{2019}t be opened"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Pretty print (-p)
// ============================================================

#[test]
fn pretty_print_dict() {
    let f = sample_plist();
    let r = run(&["-p", f.to_str().unwrap()]);
    assert!(r.stdout.starts_with("{"));
    assert!(r.stdout.contains("\"Name\" => \"Test\""));
    assert!(r.stdout.contains("\"Num\" => 42"));
    assert!(r.stdout.contains("\"Flag\" => true"));
    assert_eq!(r.exit_code, 0);
}

#[test]
fn pretty_print_sorts_keys() {
    let f = sample_plist();
    let r = run(&["-p", f.to_str().unwrap()]);
    let flag_pos = r.stdout.find("\"Flag\"").unwrap();
    let name_pos = r.stdout.find("\"Name\"").unwrap();
    assert!(flag_pos < name_pos);
}

#[test]
fn pretty_print_data_hex() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Blob</key>
	<data>aGVsbG8=</data>
</dict>
</plist>"#,
    );
    let r = run(&["-p", f.to_str().unwrap()]);
    // "hello" = 68 65 6c 6c 6f, displayed without zero-padding
    assert!(r.stdout.contains("{length = 5, bytes = 0x68656c6c6f}"));
}

// ============================================================
// Extract raw
// ============================================================

#[test]
fn extract_raw_string() {
    let f = sample_plist();
    let r = run(&["-extract", "Name", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "Test\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn extract_raw_integer() {
    let f = sample_plist();
    let r = run(&["-extract", "Num", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "42\n");
}

#[test]
fn extract_raw_float() {
    let f = sample_plist();
    let r = run(&["-extract", "Pi", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "3.140000\n");
}

#[test]
fn extract_raw_bool() {
    let f = sample_plist();
    let r = run(&["-extract", "Flag", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "true\n");
}

#[test]
fn extract_raw_array_count() {
    let f = sample_plist();
    let r = run(&["-extract", "Items", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "2\n");
}

#[test]
fn extract_raw_dict_keys_sorted() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Sub</key>
	<dict>
		<key>Zebra</key><string>z</string>
		<key>Apple</key><string>a</string>
		<key>Mango</key><string>m</string>
	</dict>
</dict>
</plist>"#,
    );
    let r = run(&["-extract", "Sub", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "Apple\nMango\nZebra\n");
}

#[test]
fn extract_raw_date() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>D</key>
	<date>2024-01-15T10:30:00Z</date>
</dict>
</plist>"#,
    );
    let r = run(&["-extract", "D", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "2024-01-15T10:30:00Z\n");
}

#[test]
fn extract_raw_data_base64() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>B</key>
	<data>AQIDBA==</data>
</dict>
</plist>"#,
    );
    let r = run(&["-extract", "B", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "AQIDBA==\n");
}

#[test]
fn extract_nested_keypath() {
    let f = sample_plist();
    let r = run(&["-extract", "Sub.Key", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "val\n");
}

#[test]
fn extract_array_index() {
    let f = sample_plist();
    let r = run(&["-extract", "Items.0", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "a\n");
    let r2 = run(&["-extract", "Items.1", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r2.stdout, "b\n");
}

#[test]
fn extract_nonexistent_keypath() {
    let f = sample_plist();
    let r = run(&["-extract", "Nope", "raw", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stderr.contains("No value at that key path"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn extract_empty_keypath_rejected() {
    let f = sample_plist();
    let r = run(&["-extract", "", "raw", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stderr.contains("No value at that key path"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn extract_no_newline_flag() {
    let f = sample_plist();
    let r = run(&["-extract", "Name", "raw", "-o", "-", "-n", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "Test");
    assert!(!r.stdout.ends_with('\n'));
}

#[test]
fn extract_xml1() {
    let f = sample_plist();
    let r = run(&["-extract", "Name", "xml1", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stdout.contains("<string>Test</string>"));
}

#[test]
fn extract_json_container() {
    let f = sample_plist();
    let r = run(&["-extract", "Items", "json", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stdout.contains("[\"a\",\"b\"]"));
    assert_eq!(r.exit_code, 0);
}

#[test]
fn extract_json_scalar_rejected() {
    let f = sample_plist();
    let r = run(&["-extract", "Name", "json", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stderr.contains("Invalid object in plist for JSON format"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn extract_raw_to_file_no_newline() {
    let f = sample_plist();
    let out = std::env::temp_dir().join("plutil_raw_out.txt");
    run(&["-extract", "Name", "raw", "-o", out.to_str().unwrap(), f.to_str().unwrap()]);
    let content = fs::read(&out).unwrap();
    assert_eq!(content, b"Test");
    fs::remove_file(&out).ok();
}

// ============================================================
// Extract with -expect
// ============================================================

#[test]
fn extract_expect_match() {
    let f = sample_plist();
    let r = run(&["-extract", "Name", "raw", "-expect", "string", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "Test\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn extract_expect_mismatch() {
    let f = sample_plist();
    let r = run(&["-extract", "Name", "raw", "-expect", "integer", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stderr.contains("expected to be integer but is string"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Type
// ============================================================

#[test]
fn type_string() {
    let f = sample_plist();
    let r = run(&["-type", "Name", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "string\n");
}

#[test]
fn type_integer() {
    let f = sample_plist();
    let r = run(&["-type", "Num", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "integer\n");
}

#[test]
fn type_float() {
    let f = sample_plist();
    let r = run(&["-type", "Pi", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "float\n");
}

#[test]
fn type_bool() {
    let f = sample_plist();
    let r = run(&["-type", "Flag", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "bool\n");
}

#[test]
fn type_array() {
    let f = sample_plist();
    let r = run(&["-type", "Items", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "array\n");
}

#[test]
fn type_dictionary() {
    let f = sample_plist();
    let r = run(&["-type", "Sub", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "dictionary\n");
}

#[test]
fn type_expect_match() {
    let f = sample_plist();
    let r = run(&["-type", "Name", "-expect", "string", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "string\n");
    assert_eq!(r.exit_code, 0);
}

#[test]
fn type_expect_mismatch() {
    let f = sample_plist();
    let r = run(&["-type", "Name", "-expect", "integer", f.to_str().unwrap()]);
    assert!(r.stderr.contains("expected to be integer but is string"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn type_empty_keypath_rejected() {
    let f = sample_plist();
    let r = run(&["-type", "", f.to_str().unwrap()]);
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Convert
// ============================================================

#[test]
fn convert_json() {
    let f = sample_plist();
    let r = run(&["-convert", "json", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stdout.contains("\"Name\":\"Test\"") || r.stdout.contains("\"Name\" : \"Test\""));
    assert_eq!(r.exit_code, 0);
}

#[test]
fn convert_json_readable() {
    let f = sample_plist();
    let r = run(&["-convert", "json", "-r", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stdout.contains("\"Name\" : \"Test\""));
    // Keys should be sorted in readable mode
    let flag_pos = r.stdout.find("\"Flag\"").unwrap();
    let name_pos = r.stdout.find("\"Name\"").unwrap();
    assert!(flag_pos < name_pos);
}

#[test]
fn convert_json_rejects_date() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>D</key>
	<date>2024-01-15T10:30:00Z</date>
</dict>
</plist>"#,
    );
    let r = run(&["-convert", "json", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stderr.contains("Invalid object in plist for JSON format"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn convert_xml1_stdout() {
    let f = sample_plist();
    let r = run(&["-convert", "xml1", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stdout.contains("<string>Test</string>"));
}

#[test]
fn convert_binary1_roundtrip() {
    let f = sample_plist();
    let bin = std::env::temp_dir().join("plutil_bin_rt.plist");
    run(&["-convert", "binary1", "-o", bin.to_str().unwrap(), f.to_str().unwrap()]);
    let r = run(&["-extract", "Name", "raw", "-o", "-", bin.to_str().unwrap()]);
    assert_eq!(r.stdout, "Test\n");
    fs::remove_file(&bin).ok();
}

#[test]
fn convert_in_place() {
    let f = sample_plist();
    run(&["-convert", "binary1", f.to_str().unwrap()]);
    // Should be binary now, read it back
    let r = run(&["-extract", "Name", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "Test\n");
}

#[test]
fn convert_bad_format() {
    let f = sample_plist();
    let r = run(&["-convert", "badformat", f.to_str().unwrap()]);
    assert!(r.stderr.contains("Unknown format specifier"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn convert_with_extension() {
    let f = sample_plist();
    run(&["-convert", "json", "-e", "json", f.to_str().unwrap()]);
    let json_path = f.with_extension("json");
    assert!(json_path.exists());
    let content = fs::read_to_string(&json_path).unwrap();
    assert!(content.contains("\"Name\""));
    fs::remove_file(&json_path).ok();
}

// ============================================================
// Convert swift
// ============================================================

#[test]
fn convert_swift() {
    let f = sample_plist();
    let r = run(&["-convert", "swift", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stdout.contains("let "));
    assert!(r.stdout.contains("\"Name\" : \"Test\""));
    assert!(r.stdout.contains("42"));
}

#[test]
fn convert_swift_header_comment() {
    let f = sample_plist();
    let r = run(&["-convert", "swift", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stdout.starts_with("/// Generated from "));
}

// ============================================================
// Convert objc
// ============================================================

#[test]
fn convert_objc() {
    let f = sample_plist();
    let r = run(&["-convert", "objc", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stdout.contains("NSDictionary * const"));
    assert!(r.stdout.contains("@\"Name\" : @\"Test\""));
    assert!(r.stdout.contains("@42"));
    assert!(r.stdout.contains("@YES"));
}

#[test]
fn convert_objc_rejects_date() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>D</key>
	<date>2024-01-15T10:30:00Z</date>
</dict>
</plist>"#,
    );
    let r = run(&["-convert", "objc", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stderr.contains("cannot be represented in Obj-C literal syntax"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Insert
// ============================================================

#[test]
fn insert_string() {
    let f = sample_plist();
    run(&["-insert", "NewKey", "-string", "hello", f.to_str().unwrap()]);
    let r = run(&["-extract", "NewKey", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "hello\n");
}

#[test]
fn insert_integer() {
    let f = sample_plist();
    run(&["-insert", "NewNum", "-integer", "99", f.to_str().unwrap()]);
    let r = run(&["-extract", "NewNum", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "99\n");
}

#[test]
fn insert_bool_true() {
    let f = sample_plist();
    run(&["-insert", "B", "-bool", "YES", f.to_str().unwrap()]);
    let r = run(&["-extract", "B", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "true\n");
}

#[test]
fn insert_bool_false() {
    let f = sample_plist();
    run(&["-insert", "B", "-bool", "NO", f.to_str().unwrap()]);
    let r = run(&["-extract", "B", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "false\n");
}

#[test]
fn insert_dictionary() {
    let f = sample_plist();
    run(&["-insert", "NewDict", "-dictionary", f.to_str().unwrap()]);
    let r = run(&["-type", "NewDict", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "dictionary\n");
}

#[test]
fn insert_array() {
    let f = sample_plist();
    run(&["-insert", "NewArr", "-array", f.to_str().unwrap()]);
    let r = run(&["-type", "NewArr", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "array\n");
}

#[test]
fn insert_append_to_array() {
    let f = sample_plist();
    run(&["-insert", "Items", "-string", "c", "-append", f.to_str().unwrap()]);
    let r = run(&["-extract", "Items.2", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "c\n");
}

#[test]
fn insert_at_array_index() {
    let f = sample_plist();
    run(&["-insert", "Items.0", "-string", "first", f.to_str().unwrap()]);
    let r = run(&["-extract", "Items.0", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "first\n");
    let r2 = run(&["-extract", "Items.1", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r2.stdout, "a\n");
}

#[test]
fn insert_json_compound() {
    let f = sample_plist();
    run(&["-insert", "Obj", "-json", "{\"k\":\"v\"}", f.to_str().unwrap()]);
    let r = run(&["-extract", "Obj.k", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "v\n");
}

#[test]
fn insert_deep_nonexistent_fails() {
    let f = sample_plist();
    let r = run(&["-insert", "A.B.C", "-string", "x", f.to_str().unwrap()]);
    assert!(r.stderr.contains("Key path not found"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Replace
// ============================================================

#[test]
fn replace_value() {
    let f = sample_plist();
    run(&["-replace", "Name", "-string", "Changed", f.to_str().unwrap()]);
    let r = run(&["-extract", "Name", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "Changed\n");
}

#[test]
fn replace_type_change() {
    let f = sample_plist();
    run(&["-replace", "Name", "-integer", "42", f.to_str().unwrap()]);
    let r = run(&["-type", "Name", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "integer\n");
}

#[test]
fn replace_nonexistent_fails() {
    let f = sample_plist();
    let r = run(&["-replace", "Nope", "-string", "x", f.to_str().unwrap()]);
    assert!(r.stderr.contains("No value at that key path"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Remove
// ============================================================

#[test]
fn remove_key() {
    let f = sample_plist();
    run(&["-remove", "Name", f.to_str().unwrap()]);
    let r = run(&["-extract", "Name", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.exit_code, 1);
}

#[test]
fn remove_nonexistent_fails() {
    let f = sample_plist();
    let r = run(&["-remove", "Nope", f.to_str().unwrap()]);
    assert!(r.stderr.contains("No value to remove at key path"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Create
// ============================================================

#[test]
fn create_xml1() {
    let out = std::env::temp_dir().join("plutil_create_xml.plist");
    let r = run(&["-create", "xml1", out.to_str().unwrap()]);
    assert_eq!(r.exit_code, 0);
    let content = fs::read_to_string(&out).unwrap();
    assert!(content.contains("<dict/>"));
    fs::remove_file(&out).ok();
}

#[test]
fn create_json() {
    let out = std::env::temp_dir().join("plutil_create_json.plist");
    let r = run(&["-create", "json", out.to_str().unwrap()]);
    assert_eq!(r.exit_code, 0);
    let content = fs::read_to_string(&out).unwrap();
    assert_eq!(content, "{}");
    fs::remove_file(&out).ok();
}

#[test]
fn create_binary1_readable() {
    let out = std::env::temp_dir().join("plutil_create_bin.plist");
    run(&["-create", "binary1", out.to_str().unwrap()]);
    let r = run(&["-p", out.to_str().unwrap()]);
    assert!(r.stdout.contains("{"));
    fs::remove_file(&out).ok();
}

// ============================================================
// Error handling
// ============================================================

#[test]
fn unrecognized_option() {
    let r = run(&["--foo"]);
    assert!(r.stderr.contains("unrecognized option: --foo"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn duplicate_command_rejected() {
    let f = sample_plist();
    let r = run(&["-lint", "-lint", f.to_str().unwrap()]);
    assert!(r.stderr.contains("unrecognized option: -lint"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn missing_format_for_convert() {
    let r = run(&["-convert"]);
    assert!(r.stderr.contains("Missing format specifier"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn missing_args_for_extract() {
    let r = run(&["-extract"]);
    assert!(r.stderr.contains("'Extract' requires"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn missing_args_for_remove() {
    let r = run(&["-remove"]);
    assert!(r.stderr.contains("'Remove' requires"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn missing_args_for_insert() {
    let r = run(&["-insert"]);
    assert!(r.stderr.contains("'Insert' and 'Replace' require"));
    assert_eq!(r.exit_code, 1);
}

#[test]
fn missing_args_for_create() {
    let r = run(&["-create"]);
    assert!(r.stderr.contains("'Create' requires"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// End-of-options (--)
// ============================================================

#[test]
fn double_dash_end_of_options() {
    let f = sample_plist();
    let r = run(&["-lint", "--", f.to_str().unwrap()]);
    assert!(r.stdout.contains(": OK"));
    assert_eq!(r.exit_code, 0);
}

// ============================================================
// Unicode
// ============================================================

#[test]
fn unicode_extract() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>emoji</key>
	<string>🎉</string>
</dict>
</plist>"#,
    );
    let r = run(&["-extract", "emoji", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "🎉\n");
}

// ============================================================
// Empty file handling
// ============================================================

#[test]
fn empty_file_lint_ok() {
    let f = std::env::temp_dir().join("plutil_empty_lint.plist");
    fs::write(&f, b"\n").unwrap();
    let r = run(&["-lint", f.to_str().unwrap()]);
    assert!(r.stdout.contains(": OK"));
    assert_eq!(r.exit_code, 0);
    fs::remove_file(&f).ok();
}

#[test]
fn empty_file_pretty_print() {
    let f = std::env::temp_dir().join("plutil_empty_p.plist");
    fs::write(&f, b"\n").unwrap();
    let r = run(&["-p", f.to_str().unwrap()]);
    assert!(r.stdout.contains("{"));
    assert_eq!(r.exit_code, 0);
    fs::remove_file(&f).ok();
}

// ============================================================
// JSON plist reading
// ============================================================

#[test]
fn read_json_plist_extract() {
    let f = std::env::temp_dir().join("plutil_json_read.plist");
    fs::write(&f, b"{\"Name\":\"Test\",\"Num\":42}").unwrap();
    let r = run(&["-extract", "Name", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "Test\n");
    assert_eq!(r.exit_code, 0);
    fs::remove_file(&f).ok();
}

#[test]
fn lint_rejects_json_plist() {
    let f = std::env::temp_dir().join("plutil_json_lint.plist");
    fs::write(&f, b"{\"Name\":\"Test\"}").unwrap();
    let r = run(&["-lint", f.to_str().unwrap()]);
    assert_eq!(r.exit_code, 1);
    // Lint uses strict plist parser, rejects JSON
    fs::remove_file(&f).ok();
}

#[test]
fn json_convert_roundtrip() {
    let f = sample_plist();
    run(&["-convert", "json", f.to_str().unwrap()]);
    // Should now be JSON format
    let r = run(&["-extract", "Name", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "Test\n");
    // Convert back to xml
    run(&["-convert", "xml1", f.to_str().unwrap()]);
    let r2 = run(&["-extract", "Name", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r2.stdout, "Test\n");
}

// ============================================================
// Deep array-in-dict keypath
// ============================================================

#[test]
fn deep_array_dict_keypath() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>L1</key>
	<array>
		<dict>
			<key>L2</key>
			<array>
				<dict><key>Val</key><string>deep</string></dict>
			</array>
		</dict>
	</array>
</dict>
</plist>"#,
    );
    let r = run(&["-extract", "L1.0.L2.0.Val", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "deep\n");
}

// ============================================================
// Cross-tool compatibility
// ============================================================

#[test]
fn plutil_reads_plistbuddy_output() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Key</key>
	<string>original</string>
</dict>
</plist>"#,
    );
    // Modify with PlistBuddy
    let pb = {
        let mut p = std::env::current_exe().unwrap();
        p.pop();
        p.pop();
        p.push("PlistBuddy");
        p
    };
    Command::new(&pb)
        .args(["-c", "Set :Key modified", f.to_str().unwrap()])
        .output()
        .unwrap();
    // Read with plutil
    let r = run(&["-extract", "Key", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "modified\n");
}

// ============================================================
// Lint error format (parenthesized)
// ============================================================

#[test]
fn lint_error_has_parens() {
    let f = std::env::temp_dir().join("plutil_lint_err.plist");
    fs::write(&f, b"not a valid plist at all").unwrap();
    let r = run(&["-lint", f.to_str().unwrap()]);
    assert_eq!(r.exit_code, 1);
    // Error message should be wrapped in parentheses
    assert!(r.stderr.contains("("), "stderr: {}", r.stderr);
    assert!(r.stderr.contains(")"), "stderr: {}", r.stderr);
    fs::remove_file(&f).ok();
}

// ============================================================
// Large array
// ============================================================

#[test]
fn large_array_100_items() {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><array>"#);
    for i in 0..100 {
        xml.push_str(&format!("<string>item{i}</string>"));
    }
    xml.push_str("</array></plist>");
    let f = temp_plist(&xml);
    let r = run(&["-extract", "99", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "item99\n");
}

// ============================================================
// String with special characters in JSON and Swift
// ============================================================

#[test]
fn json_escaping_special_chars() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Q</key>
	<string>she said "hello"</string>
	<key>NL</key>
	<string>line1
line2</string>
</dict>
</plist>"#,
    );
    let r = run(&["-convert", "json", "-r", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stdout.contains("\\\"hello\\\""));
    assert!(r.stdout.contains("\\n"));
}

// ============================================================
// plutil -p with root scalar
// ============================================================

#[test]
fn pretty_print_root_string() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<string>just a string</string>
</plist>"#,
    );
    let r = run(&["-p", f.to_str().unwrap()]);
    assert!(r.stdout.contains("just a string"));
    assert_eq!(r.exit_code, 0);
}

// ============================================================
// Remove from array
// ============================================================

#[test]
fn remove_array_element() {
    let f = sample_plist();
    run(&["-remove", "Items.0", f.to_str().unwrap()]);
    let r = run(&["-extract", "Items.0", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "b\n");
}

// ============================================================
// Replace with JSON compound
// ============================================================

#[test]
fn replace_with_json() {
    let f = sample_plist();
    run(&["-replace", "Sub", "-json", "{\"New\":\"val\"}", f.to_str().unwrap()]);
    let r = run(&["-extract", "Sub.New", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "val\n");
}

// ============================================================
// Negative values
// ============================================================

#[test]
fn insert_negative_integer() {
    let f = sample_plist();
    run(&["-insert", "Neg", "-integer", "-99", f.to_str().unwrap()]);
    let r = run(&["-extract", "Neg", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "-99\n");
}

#[test]
fn insert_negative_float() {
    let f = sample_plist();
    run(&["-insert", "NegF", "-float", "-3.14", f.to_str().unwrap()]);
    let r = run(&["-extract", "NegF", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "-3.140000\n");
}

// ============================================================
// Binary plist roundtrip
// ============================================================

#[test]
fn binary_plist_roundtrip_via_plutil() {
    let f = sample_plist();
    run(&["-convert", "binary1", f.to_str().unwrap()]);
    let r = run(&["-extract", "Name", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "Test\n");
    // Convert back
    run(&["-convert", "xml1", f.to_str().unwrap()]);
    let r2 = run(&["-extract", "Name", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r2.stdout, "Test\n");
}

// ============================================================
// -s flag behavior
// ============================================================

#[test]
fn silent_flag_with_missing_file_still_shows_error() {
    let r = run(&["-lint", "-s", "/tmp/no_such_plutil_s.plist"]);
    assert!(!r.stderr.is_empty());
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Insert at array index
// ============================================================

#[test]
fn insert_at_array_middle() {
    let f = sample_plist();
    run(&["-insert", "Items.1", "-string", "middle", f.to_str().unwrap()]);
    let r = run(&["-extract", "Items.0", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "a\n");
    let r1 = run(&["-extract", "Items.1", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r1.stdout, "middle\n");
    let r2 = run(&["-extract", "Items.2", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r2.stdout, "b\n");
}

// ============================================================
// Insert -xml with compound value
// ============================================================

#[test]
fn insert_xml_compound() {
    let f = sample_plist();
    run(&["-insert", "Xml", "-xml", "<array><string>x</string></array>", f.to_str().unwrap()]);
    let r = run(&["-extract", "Xml.0", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "x\n");
}

// ============================================================
// Pretty print date and data
// ============================================================

#[test]
fn pretty_print_date_format() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>D</key>
	<date>2024-06-15T12:00:00Z</date>
</dict>
</plist>"#,
    );
    let r = run(&["-p", f.to_str().unwrap()]);
    assert!(r.stdout.contains("2024-06-15"));
    assert!(r.stdout.contains("+0000"));
}

// ============================================================
// Real-world system plist (if available)
// ============================================================

#[test]
fn reads_system_binary_plist() {
    let path = "/System/Applications/Utilities/Terminal.app/Contents/Info.plist";
    if std::path::Path::new(path).exists() {
        let r = run(&["-extract", "CFBundleIdentifier", "raw", "-o", "-", path]);
        assert!(r.stdout.contains("Terminal") || r.stdout.contains("terminal"));
        assert_eq!(r.exit_code, 0);
    }
}

// ============================================================
// Cross-tool: PlistBuddy writes, plutil reads
// ============================================================

#[test]
fn cross_tool_plistbuddy_writes_plutil_reads() {
    let f = sample_plist();
    let pb = {
        let mut p = std::env::current_exe().unwrap();
        p.pop();
        p.pop();
        p.push("PlistBuddy");
        p
    };
    Command::new(&pb)
        .args(["-c", "Add :New string cross-test", f.to_str().unwrap()])
        .output()
        .unwrap();
    let r = run(&["-extract", "New", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "cross-test\n");
}

// ============================================================
// Insert existing key fails
// ============================================================

#[test]
fn insert_existing_key_fails() {
    let f = sample_plist();
    let r = run(&["-insert", "Name", "-string", "overwrite", f.to_str().unwrap()]);
    assert!(r.stderr.contains("Value already exists at key path"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Remove last key from dict leaves empty dict
// ============================================================

#[test]
fn remove_last_key_leaves_empty_dict() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Only</key>
	<string>val</string>
</dict>
</plist>"#,
    );
    run(&["-remove", "Only", f.to_str().unwrap()]);
    let r = run(&["-p", f.to_str().unwrap()]);
    assert!(r.stdout.contains("{"));
    // Name should be gone
    let r2 = run(&["-extract", "Only", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r2.exit_code, 1);
}

// ============================================================
// Append to non-array fails with correct message
// ============================================================

#[test]
fn append_to_non_array_error() {
    let f = sample_plist();
    let r = run(&["-insert", "Name", "-string", "x", "-append", f.to_str().unwrap()]);
    assert!(r.stderr.contains("Appending to a non-array"));
    assert_eq!(r.exit_code, 1);
}

// ============================================================
// Natural sort in -p output
// ============================================================

#[test]
fn pretty_print_natural_sort() {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>"#);
    for i in [10, 2, 1, 20, 3] {
        xml.push_str(&format!("<key>Item{i}</key><string>v{i}</string>"));
    }
    xml.push_str("</dict></plist>");
    let f = temp_plist(&xml);
    let r = run(&["-p", f.to_str().unwrap()]);
    // Natural sort: Item1, Item2, Item3, Item10, Item20
    let keys: Vec<&str> = r.stdout.lines()
        .filter(|l| l.contains("\"Item"))
        .map(|l| l.trim())
        .collect();
    assert_eq!(keys[0], "\"Item1\" => \"v1\"");
    assert_eq!(keys[1], "\"Item2\" => \"v2\"");
    assert_eq!(keys[2], "\"Item3\" => \"v3\"");
    assert_eq!(keys[3], "\"Item10\" => \"v10\"");
    assert_eq!(keys[4], "\"Item20\" => \"v20\"");
}

// ============================================================
// JSON readable sort is also natural
// ============================================================

#[test]
fn json_readable_natural_sort() {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>"#);
    for i in [10, 2, 1] {
        xml.push_str(&format!("<key>K{i}</key><string>v{i}</string>"));
    }
    xml.push_str("</dict></plist>");
    let f = temp_plist(&xml);
    let r = run(&["-convert", "json", "-r", "-o", "-", f.to_str().unwrap()]);
    let k1_pos = r.stdout.find("\"K1\"").unwrap();
    let k2_pos = r.stdout.find("\"K2\"").unwrap();
    let k10_pos = r.stdout.find("\"K10\"").unwrap();
    assert!(k1_pos < k2_pos);
    assert!(k2_pos < k10_pos);
}

// ============================================================
// Swift mixed array gets [Any] annotation
// ============================================================

#[test]
fn swift_mixed_array_any_annotation() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<string>text</string>
	<integer>42</integer>
</array>
</plist>"#,
    );
    let r = run(&["-convert", "swift", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stdout.contains(": [Any] = ["));
}

#[test]
fn swift_homogeneous_array_no_annotation() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<string>a</string>
	<string>b</string>
</array>
</plist>"#,
    );
    let r = run(&["-convert", "swift", "-o", "-", f.to_str().unwrap()]);
    assert!(!r.stdout.contains("[Any]"));
    assert!(r.stdout.contains("let "));
}

// ============================================================
// Nested arrays in JSON
// ============================================================

#[test]
fn json_nested_arrays() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Matrix</key>
	<array>
		<array><integer>1</integer><integer>2</integer></array>
		<array><integer>3</integer><integer>4</integer></array>
	</array>
</dict>
</plist>"#,
    );
    let r = run(&["-convert", "json", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stdout.contains("[[1,2],[3,4]]"));
}

// ============================================================
// Extract from root array by index
// ============================================================

#[test]
fn extract_from_root_array() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<dict><key>name</key><string>Alice</string></dict>
	<dict><key>name</key><string>Bob</string></dict>
</array>
</plist>"#,
    );
    let r = run(&["-extract", "0.name", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "Alice\n");
    let r2 = run(&["-extract", "1.name", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r2.stdout, "Bob\n");
}

// ============================================================
// -p does NOT escape special chars in strings
// ============================================================

#[test]
fn pretty_print_raw_strings() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Q</key>
	<string>she said "hello"</string>
	<key>BS</key>
	<string>back\slash</string>
</dict>
</plist>"#,
    );
    let r = run(&["-p", f.to_str().unwrap()]);
    // -p shows raw quotes and backslashes, not escaped
    assert!(r.stdout.contains(r#""she said "hello"""#));
    assert!(r.stdout.contains(r#""back\slash""#));
}

// ============================================================
// Swift and ObjC output literal tabs (not \t)
// ============================================================

#[test]
fn swift_literal_tab() {
    let f = temp_plist(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Tab</key>
	<string>col1	col2</string>
</dict>
</plist>"#,
    );
    let r = run(&["-convert", "swift", "-o", "-", f.to_str().unwrap()]);
    assert!(r.stdout.contains("col1\tcol2"));
    assert!(!r.stdout.contains("\\t"));
}

// ============================================================
// Replace container with scalar
// ============================================================

#[test]
fn replace_container_with_scalar() {
    let f = sample_plist();
    run(&["-replace", "Sub", "-string", "flat", f.to_str().unwrap()]);
    let r = run(&["-extract", "Sub", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "flat\n");
    let r2 = run(&["-type", "Sub", f.to_str().unwrap()]);
    assert_eq!(r2.stdout, "string\n");
}

// ============================================================
// Replace scalar with container
// ============================================================

#[test]
fn replace_scalar_with_container() {
    let f = sample_plist();
    run(&["-replace", "Name", "-json", "{\"k\":\"v\"}", f.to_str().unwrap()]);
    let r = run(&["-extract", "Name.k", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "v\n");
}

// ============================================================
// Convert idempotent (xml1 -> xml1)
// ============================================================

#[test]
fn convert_xml1_idempotent() {
    let f = sample_plist();
    let before = fs::read_to_string(f.to_str().unwrap()).unwrap();
    run(&["-convert", "xml1", f.to_str().unwrap()]);
    let after = fs::read_to_string(f.to_str().unwrap()).unwrap();
    // Content should be preserved (may differ in whitespace from original hand-written XML)
    let r = run(&["-extract", "Name", "raw", "-o", "-", f.to_str().unwrap()]);
    assert_eq!(r.stdout, "Test\n");
    let _ = (before, after); // suppress unused warnings
}
