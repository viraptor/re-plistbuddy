use anyhow::{bail, Context, Result};
use crate::value::{Dictionary, Value};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

const USAGE: &str = "\
Usage: PlistBuddy [-cxlh] <file.plist>
    -c \"<command>\" execute command, otherwise run in interactive mode
    -x output will be in the form of an xml plist where appropriate
    -l if the path to <file.plist> contains symbolic links, they will
       not be followed.
    -h print the complete help info, with command guide
";

const HELP_TEXT: &str = "\
Command Format:
    Help - Prints this information
    Exit - Exits the program, changes are not saved to the file
    Save - Saves the current changes to the file
    Revert - Reloads the last saved version of the file
    Clear [<Type>] - Clears out all existing entries, and creates root of Type
    Print [<Entry>] - Prints value of Entry.  Otherwise, prints file
    Set <Entry> <Value> - Sets the value at Entry to Value
    Add <Entry> <Type> [<Value>] - Adds Entry to the plist, with value Value
    Copy <EntrySrc> <EntryDst> - Copies the EntrySrc property to EntryDst
    Delete <Entry> - Deletes Entry from the plist
    Merge <file.plist> [<Entry>] - Adds the contents of file.plist to Entry
    Import <Entry> <file> - Creates or sets Entry the contents of file

Entry Format:
    Entries consist of property key names delimited by colons.  Array items
    are specified by a zero-based integer index.  Examples:
        :CFBundleShortVersionString
        :CFBundleDocumentTypes:2:CFBundleTypeExtensions

Types:
    string
    array
    dict
    bool
    real
    integer
    date
    data

Examples:
    Set :CFBundleIdentifier com.apple.plistbuddy
        Sets the CFBundleIdentifier property to com.apple.plistbuddy
    Add :CFBundleGetInfoString string \"App version 1.0.1\"
        Adds the CFBundleGetInfoString property to the plist
    Add :CFBundleDocumentTypes: dict
        Adds a new item of type dict to the CFBundleDocumentTypes array
    Add :CFBundleDocumentTypes:0 dict
        Adds the new item to the beginning of the array
    Delete :CFBundleDocumentTypes:0 dict
        Deletes the FIRST item in the array
    Delete :CFBundleDocumentTypes
        Deletes the ENTIRE CFBundleDocumentTypes array
";

struct PlistState {
    root: Value,
    file_path: PathBuf,
    xml_output: bool,
    dirty: bool,
}

impl PlistState {
    fn load(file_path: PathBuf, no_follow_symlinks: bool) -> Result<Self> {
        if no_follow_symlinks {
            // Check if the file path itself is a symlink
            let metadata = std::fs::symlink_metadata(&file_path);
            if let Ok(meta) = metadata {
                if meta.is_symlink() {
                    // Resolve the parent dir but keep the filename for the error message
                    let display_path = if let Some(parent) = file_path.parent() {
                        std::fs::canonicalize(parent)
                            .unwrap_or(parent.to_path_buf())
                            .join(file_path.file_name().unwrap_or_default())
                    } else {
                        file_path.clone()
                    };
                    eprintln!(
                        "Error Opening File: {} [Too many levels of symbolic links]",
                        display_path.display()
                    );
                    bail!("Error Reading File: {}", display_path.display());
                }
            }
        }

        let resolved = if no_follow_symlinks {
            file_path.clone()
        } else {
            // Try full canonicalize first; if file doesn't exist, canonicalize
            // the parent directory and append the filename (matching Apple's behavior)
            std::fs::canonicalize(&file_path).unwrap_or_else(|_| {
                if let Some(parent) = file_path.parent() {
                    if let Ok(canonical_parent) = std::fs::canonicalize(parent) {
                        if let Some(name) = file_path.file_name() {
                            return canonical_parent.join(name);
                        }
                    }
                }
                file_path.clone()
            })
        };

        let root = if resolved.exists() {
            match Value::from_file(&resolved) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{e}");
                    println!("Error Reading File: {}", resolved.display());
                    bail!("");
                }
            }
        } else {
            println!(
                "File Doesn't Exist, Will Create: {}",
                resolved.display()
            );
            Value::Dictionary(Dictionary::new())
        };

        Ok(PlistState {
            root,
            file_path: resolved,
            xml_output: false,
            dirty: false,
        })
    }

    fn save(&self) -> Result<()> {
        self.root
            .to_file_xml(&self.file_path)
            .with_context(|| format!("Error Writing File: {}", self.file_path.display()))
    }

    fn revert(&mut self) -> Result<()> {
        self.root = Value::from_file(&self.file_path)
            .with_context(|| format!("Error Reading File: {}", self.file_path.display()))?;
        self.dirty = false;
        Ok(())
    }

    fn mutated(&mut self) -> CommandResult {
        self.dirty = true;
        CommandResult::Ok
    }
}

// Returns true if any command failed
pub fn run(args: &[String]) -> Result<bool> {
    let mut commands: Vec<String> = Vec::new();
    let mut xml_output = false;
    let mut no_follow_symlinks = false;
    let mut show_help = false;
    let mut file_path: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "-c" {
            i += 1;
            if i >= args.len() {
                println!("{USAGE}");
                bail!("");
            }
            commands.push(args[i].clone());
        } else if arg == "-x" {
            xml_output = true;
        } else if arg == "-l" {
            no_follow_symlinks = true;
        } else if arg == "-h" {
            show_help = true;
        } else {
            if file_path.is_none() {
                file_path = Some(arg.clone());
            }
            // Extra positional args are handled by the check below
        }
        i += 1;
    }

    // Check for extra positional arguments (PlistBuddy only accepts one file path)
    {
        let mut positional_count = 0;
        let mut j = 0;
        while j < args.len() {
            if args[j] == "-c" {
                j += 1; // skip the command argument
            } else if args[j] == "-x" || args[j] == "-l" || args[j] == "-h" {
                // flag
            } else {
                positional_count += 1;
            }
            j += 1;
        }
        if positional_count > 1 {
            // PlistBuddy loads the file first, then errors
            let fp = file_path.as_ref().map(|p| PathBuf::from(p));
            if let Some(ref path) = fp {
                if !path.exists() {
                    let resolved = if no_follow_symlinks {
                        path.clone()
                    } else {
                        if let Some(parent) = path.parent() {
                            std::fs::canonicalize(parent)
                                .unwrap_or(parent.to_path_buf())
                                .join(path.file_name().unwrap_or_default())
                        } else {
                            path.clone()
                        }
                    };
                    println!("File Doesn't Exist, Will Create: {}", resolved.display());
                }
            }
            println!("Invalid Arguments");
            println!();
            return Ok(true);
        }
    }

    if show_help && file_path.is_none() {
        println!("{HELP_TEXT}");
        return Ok(true);
    }

    let file_path = match file_path {
        Some(p) => PathBuf::from(p),
        None => {
            println!("{USAGE}");
            return Ok(true);
        }
    };

    if show_help {
        println!("{HELP_TEXT}");
        return Ok(false);
    }

    let mut state = PlistState::load(file_path, no_follow_symlinks)?;
    state.xml_output = xml_output;

    if commands.is_empty() {
        run_interactive(&mut state)
    } else {
        run_commands(&mut state, &commands)
    }
}

fn handle_result(state: &mut PlistState, result: CommandResult, any_failed: &mut bool) -> Result<bool> {
    match result {
        CommandResult::Ok => Ok(false),
        CommandResult::StdoutError(msg) => {
            println!("{msg}");
            *any_failed = true;
            Ok(false)
        }
        CommandResult::StderrError(msg) => {
            eprintln!("{msg}");
            *any_failed = true;
            Ok(false)
        }
        CommandResult::Exit => Ok(true),
        CommandResult::Save => {
            println!("Saving...");
            state.save()?;
            state.dirty = false;
            Ok(false)
        }
        CommandResult::Revert => {
            println!("Reverting to last saved state...");
            state.revert()?;
            Ok(false)
        }
    }
}

fn run_interactive(state: &mut PlistState) -> Result<bool> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut any_failed = false;

    loop {
        print!("Command: ");
        io::stdout().flush()?;

        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let result = execute_command(state, line);
        if handle_result(state, result, &mut any_failed)? {
            break;
        }
    }

    Ok(any_failed)
}

fn run_commands(state: &mut PlistState, commands: &[String]) -> Result<bool> {
    let mut any_failed = false;

    for cmd in commands {
        let result = execute_command(state, cmd);
        handle_result(state, result, &mut any_failed)?;
    }

    if state.dirty {
        if let Err(e) = state.save() {
            eprintln!("{e}");
        }
    }
    Ok(any_failed)
}

enum CommandResult {
    Ok,
    StdoutError(String),
    StderrError(String),
    Exit,
    Save,
    Revert,
}

fn execute_command(state: &mut PlistState, input: &str) -> CommandResult {
    let input = input.trim_end();
    if input.is_empty() {
        return CommandResult::StdoutError("Unrecognized Command".to_string());
    }

    // PlistBuddy does not accept leading whitespace before the command word
    if input.starts_with(char::is_whitespace) {
        return CommandResult::StdoutError("Unrecognized Command".to_string());
    }

    let (cmd_word, rest) = split_first_word(input);
    let cmd_lower = cmd_word.to_lowercase();

    match cmd_lower.as_str() {
        "help" => {
            println!("{HELP_TEXT}");
            CommandResult::Ok
        }
        "exit" => CommandResult::Exit,
        "save" => CommandResult::Save,
        "revert" => CommandResult::Revert,
        "clear" => cmd_clear(state, rest),
        "print" => cmd_print(state, rest),
        "set" => cmd_set(state, rest),
        "add" => cmd_add(state, rest),
        "copy" => cmd_copy(state, rest),
        "delete" => cmd_delete(state, rest),
        "merge" => cmd_merge(state, rest),
        "import" => cmd_import(state, rest),
        _ => CommandResult::StdoutError("Unrecognized Command".to_string()),
    }
}

fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim();
    match s.find(char::is_whitespace) {
        Some(pos) => (&s[..pos], s[pos..].trim_start()),
        None => (s, ""),
    }
}

/// Extract one token with quote-aware grouping, returning (token, rest).
/// Quoted strings group words and quotes are stripped.
fn next_token(s: &str) -> (String, &str) {
    let s = s.trim_start();
    if s.is_empty() {
        return (String::new(), "");
    }

    let bytes = s.as_bytes();
    let first = bytes[0];
    if first == b'"' || first == b'\'' {
        let quote = first;
        if let Some(end) = s[1..].find(|c: char| c as u8 == quote) {
            let token = &s[1..1 + end];
            let rest = &s[1 + end + 1..];
            return (token.to_string(), rest.trim_start());
        }
        // No closing quote - treat the rest as the token, stripping the opening quote
        return (s[1..].to_string(), "");
    }

    // For unquoted tokens, still strip any embedded quote characters
    match s.find(char::is_whitespace) {
        Some(pos) => (strip_quotes(&s[..pos]), s[pos..].trim_start()),
        None => (strip_quotes(s), ""),
    }
}

/// Strip quote characters and process escape sequences (for values/rest-of-line).
fn strip_quotes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' | '\'' => {}
            '\\' => match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some(other) => result.push(other),
                None => {}
            },
            c => result.push(c),
        }
    }
    result
}

fn parse_entry_path(entry: &str) -> Vec<String> {
    let entry = entry.strip_prefix(':').unwrap_or(entry);
    if entry.is_empty() {
        return Vec::new();
    }
    entry.split(':').map(|s| s.to_string()).collect()
}

fn format_entry_for_error(entry: &str) -> String {
    entry.trim().to_string()
}

fn io_error_message(e: &std::io::Error) -> String {
    if let Some(raw) = e.raw_os_error() {
        // Use strerror to get the C-style message matching Apple's output
        let cstr = unsafe { std::ffi::CStr::from_ptr(libc::strerror(raw)) };
        cstr.to_string_lossy().to_string()
    } else {
        format!("{e}")
    }
}


/// Navigate path, auto-creating intermediate dicts as needed.
/// Returns Err("cant_add") if an intermediate exists but is not a dict/array.
fn ensure_path_mut<'a>(
    root: &'a mut Value,
    path: &[String],
) -> std::result::Result<&'a mut Value, String> {
    let mut current = root;
    for component in path {
        current = match current {
            Value::Dictionary(dict) => {
                if !dict.contains_key(component.as_str()) {
                    dict.insert(
                        component.clone(),
                        Value::Dictionary(Dictionary::new()),
                    );
                }
                dict.get_mut(component.as_str()).unwrap()
            }
            Value::Array(arr) => {
                let idx: usize = component.parse().map_err(|_| "cant_add".to_string())?;
                arr.get_mut(idx).ok_or_else(|| "cant_add".to_string())?
            }
            _ => return Err("cant_add".to_string()),
        };
    }
    Ok(current)
}

fn resolve_entry<'a>(root: &'a Value, path: &[String]) -> std::result::Result<&'a Value, ()> {
    let mut current = root;
    for component in path {
        current = match current {
            Value::Dictionary(dict) => dict.get(component.as_str()).ok_or(())?,
            Value::Array(arr) => {
                let idx: usize = component.parse().map_err(|_| ())?;
                arr.get(idx).ok_or(())?
            }
            _ => return Err(()),
        };
    }
    Ok(current)
}

fn resolve_entry_mut<'a>(
    root: &'a mut Value,
    path: &[String],
) -> std::result::Result<&'a mut Value, ()> {
    let mut current = root;
    for component in path {
        current = match current {
            Value::Dictionary(dict) => dict.get_mut(component.as_str()).ok_or(())?,
            Value::Array(arr) => {
                let idx: usize = component.parse().map_err(|_| ())?;
                arr.get_mut(idx).ok_or(())?
            }
            _ => return Err(()),
        };
    }
    Ok(current)
}

// --- Print ---

fn cmd_print(state: &PlistState, args: &str) -> CommandResult {
    let (entry_tok, _) = next_token(args);

    let value = if entry_tok.is_empty() {
        &state.root
    } else {
        let path = parse_entry_path(&entry_tok);
        if path.is_empty() {
            &state.root
        } else {
            match resolve_entry(&state.root, &path) {
                Ok(v) => v,
                Err(()) => {
                    let entry_str = format_entry_for_error(&entry_tok);
                    return CommandResult::StderrError(format!(
                        "Print: Entry, \"{entry_str}\", Does Not Exist"
                    ));
                }
            }
        }
    };

    if state.xml_output {
        print_xml(value);
    } else {
        print_value(value, 0);
    }

    CommandResult::Ok
}

fn print_xml(value: &Value) {
    let buf = value.to_xml_bytes().expect("XML serialization failed");
    let s = String::from_utf8_lossy(&buf);
    print!("{s}");
    if !s.ends_with('\n') {
        println!();
    }
}

fn print_value(value: &Value, indent: usize) {
    let prefix = "    ".repeat(indent);
    match value {
        Value::String(s) => println!("{prefix}{s}"),
        Value::Integer(i) => {
            println!("{prefix}{i}");
        }
        Value::Real(f) => println!("{prefix}{}", format_real(*f)),
        Value::Boolean(b) => println!("{prefix}{b}"),
        Value::Date(d) => {
            let formatted = format_date(*d);
            println!("{prefix}{formatted}");
        }
        Value::Data(bytes) => {
            let stdout = io::stdout();
            let mut out = stdout.lock();
            if !prefix.is_empty() {
                let _ = write!(out, "{prefix}");
            }
            let _ = out.write_all(bytes);
            let _ = writeln!(out);
        }
        Value::Array(arr) => {
            println!("{prefix}Array {{");
            for item in arr {
                print_value(item, indent + 1);
            }
            println!("{prefix}}}");
        }
        Value::Dictionary(dict) => {
            println!("{prefix}Dict {{");
            for (key, val) in dict.iter() {
                match val {
                    Value::Array(_) | Value::Dictionary(_) => {
                        let inner_prefix = "    ".repeat(indent + 1);
                        print!("{inner_prefix}{key} = ");
                        print_value_inline(val, indent + 1);
                    }
                    _ => {
                        let inner_prefix = "    ".repeat(indent + 1);
                        print!("{inner_prefix}{key} = ");
                        print_scalar_inline(val);
                    }
                }
            }
            println!("{prefix}}}");
        }
    }
}

fn print_value_inline(value: &Value, indent: usize) {
    match value {
        Value::Array(arr) => {
            println!("Array {{");
            for item in arr {
                print_value(item, indent + 1);
            }
            let prefix = "    ".repeat(indent);
            println!("{prefix}}}");
        }
        Value::Dictionary(dict) => {
            println!("Dict {{");
            for (key, val) in dict.iter() {
                match val {
                    Value::Array(_) | Value::Dictionary(_) => {
                        let inner_prefix = "    ".repeat(indent + 1);
                        print!("{inner_prefix}{key} = ");
                        print_value_inline(val, indent + 1);
                    }
                    _ => {
                        let inner_prefix = "    ".repeat(indent + 1);
                        print!("{inner_prefix}{key} = ");
                        print_scalar_inline(val);
                    }
                }
            }
            let prefix = "    ".repeat(indent);
            println!("{prefix}}}");
        }
        _ => print_scalar_inline(value),
    }
}

fn print_scalar_inline(value: &Value) {
    match value {
        Value::String(s) => println!("{s}"),
        Value::Integer(i) => {
            println!("{i}");
        }
        Value::Real(f) => println!("{}", format_real(*f)),
        Value::Boolean(b) => println!("{b}"),
        Value::Date(d) => {
            let formatted = format_date(*d);
            println!("{formatted}");
        }
        Value::Data(bytes) => {
            let stdout = io::stdout();
            let mut out = stdout.lock();
            let _ = out.write_all(bytes);
            let _ = writeln!(out);
        }
        Value::Array(_) | Value::Dictionary(_) => {}
    }
}

fn format_real(v: f64) -> String {
    crate::cf::format_double_6f(v)
}

fn format_date(abs_time: crate::value::AbsoluteTime) -> String {
    // Convert CFAbsoluteTime to Unix timestamp
    let unix_ts = (abs_time + crate::value::CF_EPOCH_OFFSET) as i64;
    format_timestamp_local(unix_ts)
}

fn format_timestamp_local(utc_ts: i64) -> String {
    // PlistBuddy always displays the standard (non-DST) timezone abbreviation
    // even when DST is active. We match this by using tzname[0].
    #[cfg(unix)]
    {
        use std::ffi::CStr;

        unsafe extern "C" {
            static tzname: [*const libc::c_char; 2];
        }

        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        let time_t = utc_ts as libc::time_t;
        unsafe {
            libc::localtime_r(&time_t, &mut tm);
        }

        // localtime_r calls tzset() internally, so tzname is populated
        let std_tz = unsafe {
            CStr::from_ptr(tzname[0]).to_string_lossy().to_string()
        };

        let mut buf = [0u8; 128];
        let fmt = b"%a %b %d %H:%M:%S \0";
        let fmt_cstr = unsafe { CStr::from_bytes_with_nul_unchecked(fmt) };
        let len = unsafe {
            libc::strftime(
                buf.as_mut_ptr() as *mut libc::c_char,
                buf.len(),
                fmt_cstr.as_ptr(),
                &tm,
            )
        };
        let datetime_part = String::from_utf8_lossy(&buf[..len]).to_string();

        format!("{datetime_part}{std_tz} {}", 1900 + tm.tm_year)
    }

    #[cfg(not(unix))]
    {
        format!("timestamp:{utc_ts}")
    }
}

fn parse_date_input(s: &str) -> Option<crate::value::AbsoluteTime> {
    #[cfg(unix)]
    {
        use std::ffi::CString;

        let formats = [
            "%a %b %d %H:%M:%S %Z %Y",
            "%a %b %d %H:%M:%S %Y",
        ];

        let input = CString::new(s).ok()?;

        for fmt in &formats {
            let fmt_c = CString::new(*fmt).ok()?;
            let mut tm: libc::tm = unsafe { std::mem::zeroed() };
            tm.tm_isdst = -1;

            let result = unsafe {
                libc::strptime(input.as_ptr(), fmt_c.as_ptr(), &mut tm)
            };

            if !result.is_null() {
                let time_t = unsafe { libc::mktime(&mut tm) };
                if time_t == -1 {
                    continue;
                }
                // Convert Unix time_t to CFAbsoluteTime
                return Some(time_t as f64 - crate::value::CF_EPOCH_OFFSET);
            }
        }
        None
    }

    #[cfg(not(unix))]
    {
        let _ = s;
        None
    }
}

// --- Clear ---

fn cmd_clear(state: &mut PlistState, args: &str) -> CommandResult {
    let type_str = args.trim();

    let new_root = match parse_type_for_clear(type_str) {
        Some(v) => {
            println!("Initializing Plist...");
            v
        }
        None => {
            println!("Unrecognized Type: {type_str}");
            println!("Initializing Plist...");
            Value::Dictionary(Dictionary::new())
        }
    };

    state.root = new_root;
    state.mutated()
}

fn parse_type_for_clear(type_str: &str) -> Option<Value> {
    match type_str.to_lowercase().as_str() {
        "dict" => Some(Value::Dictionary(Dictionary::new())),
        "array" => Some(Value::Array(Vec::new())),
        "string" => Some(Value::String(String::new())),
        "integer" => Some(Value::Integer(0)),
        "real" => Some(Value::Real(0.0)),
        "bool" => Some(Value::Boolean(false)),
        "date" => Some(Value::Date(0.0)),
        "data" => Some(Value::Data(Vec::new())),
        _ => None,
    }
}

// --- Set ---

fn cmd_set(state: &mut PlistState, args: &str) -> CommandResult {
    let (entry_str, rest) = next_token(args);
    let value_str = strip_quotes(rest);

    let path = parse_entry_path(&entry_str);

    let target = if path.is_empty() {
        &mut state.root
    } else {
        match resolve_entry_mut(&mut state.root, &path) {
            Ok(v) => v,
            Err(()) => {
                return CommandResult::StderrError(format!(
                    "Set: Entry, \"{}\", Does Not Exist",
                    format_entry_for_error(&entry_str)
                ));
            }
        }
    };

    match target {
        Value::Dictionary(_) | Value::Array(_) => {
            return CommandResult::StderrError("Set: Cannot Perform Set On Containers".to_string());
        }
        _ => {}
    }

    let new_value = coerce_value(target, &value_str);
    *target = new_value;
    state.mutated()
}

fn coerce_value(existing: &Value, input: &str) -> Value {
    match existing {
        Value::String(_) => Value::String(input.to_string()),
        Value::Integer(_) => {
            if let Ok(v) = input.parse::<i64>() {
                Value::Integer(v)
            } else if let Ok(v) = input.parse::<f64>() {
                Value::Integer(v as i64)
            } else {
                println!("Unrecognized Integer Format");
                existing.clone()
            }
        }
        Value::Real(_) => {
            if let Ok(v) = input.parse::<f64>() {
                Value::Real(v)
            } else {
                println!("Unrecognized Real Format");
                existing.clone()
            }
        }
        Value::Boolean(_) => {
            let v = matches!(input.to_lowercase().as_str(), "true" | "yes" | "1");
            Value::Boolean(v)
        }
        Value::Date(_) => {
            if let Some(d) = parse_date_input(input) {
                Value::Date(d)
            } else {
                println!("Unrecognized Date Format");
                existing.clone()
            }
        }
        Value::Data(_) => Value::Data(input.as_bytes().to_vec()),
        _ => Value::String(input.to_string()),
    }
}

// --- Add ---

fn cmd_add(state: &mut PlistState, args: &str) -> CommandResult {
    let (entry_str, rest) = next_token(args);

    let (type_str, value_rest) = next_token(rest);

    let value_str = strip_quotes(value_rest);
    let new_value = match make_value_from_type(&type_str, &value_str) {
        Ok(Some(v)) => v,
        Ok(None) => return CommandResult::Ok,
        Err(msg) => return CommandResult::StdoutError(msg),
    };

    // Check for trailing colon = array append
    let is_array_append = entry_str.ends_with(':');

    let path = parse_entry_path(&entry_str);

    if is_array_append {
        // For paths like ":Tags:", path includes empty last element; for ":", path is empty
        let parent_path = if path.is_empty() {
            &[][..]
        } else {
            &path[..path.len() - 1]
        };
        if parent_path.is_empty() {
            match &mut state.root {
                Value::Array(arr) => {
                    arr.push(new_value);
                    return state.mutated();
                }
                Value::Dictionary(dict) => {
                    if path.is_empty() || path.last().is_some_and(|s| s.is_empty()) {
                        let key = "".to_string();
                        if dict.contains_key(&key) {
                            return CommandResult::StderrError(format!(
                                "Add: \"{}\" Entry Already Exists",
                                format_entry_for_error(&entry_str)
                            ));
                        }
                        dict.insert(key, new_value);
                        return state.mutated();
                    }
                    return CommandResult::StderrError(format!(
                        "Add: \"{}\", Entry Already Exists",
                        format_entry_for_error(&entry_str)
                    ));
                }
                _ => {
                    return CommandResult::StderrError(format!(
                        "Add: \"{}\", Entry Already Exists",
                        format_entry_for_error(&entry_str)
                    ));
                }
            }
        }
        let target = match resolve_entry_mut(&mut state.root, parent_path) {
            Ok(v) => v,
            Err(()) => {
                return CommandResult::StderrError(format!(
                    "Add: Entry, \"{}\", Does Not Exist",
                    format_entry_for_error(&entry_str)
                ));
            }
        };
        match target {
            Value::Array(arr) => {
                arr.push(new_value);
                state.mutated()
            }
            _ => CommandResult::StderrError(format!(
                "Add: Entry, \"{}\", Does Not Exist",
                format_entry_for_error(&entry_str)
            )),
        }
    } else {
        if path.is_empty() {
            return CommandResult::StderrError(format!(
                "Add: \"{}\" Entry Already Exists",
                format_entry_for_error(&entry_str)
            ));
        }
        // Check if the last component is an array index for insertion
        let last = &path[path.len() - 1];
        let is_index = last.parse::<usize>().is_ok();

        if path.len() == 1 {
            // Adding to root
            match &mut state.root {
                Value::Dictionary(dict) => {
                    let key = &path[0];
                    if dict.contains_key(key) {
                        return CommandResult::StderrError(format!(
                            "Add: \"{}\" Entry Already Exists",
                            format_entry_for_error(&entry_str)
                        ));
                    }
                    dict.insert(key.clone(), new_value);
                    state.mutated()
                }
                Value::Array(arr) => {
                    if is_index {
                        let idx: usize = last.parse().unwrap_or(0);
                        let idx = idx.min(arr.len());
                        arr.insert(idx, new_value);
                        state.mutated()
                    } else {
                        CommandResult::StderrError(format!(
                            "Add: Entry, \"{}\", Does Not Exist",
                            format_entry_for_error(&entry_str)
                        ))
                    }
                }
                _ => CommandResult::StderrError(format!(
                    "Add: Can't Add Entry, \"{}\", to Parent",
                    format_entry_for_error(&entry_str)
                )),
            }
        } else {
            let parent_path = &path[..path.len() - 1];
            let parent = match ensure_path_mut(&mut state.root, parent_path) {
                Ok(v) => v,
                Err(_) => {
                    return CommandResult::StderrError(format!(
                        "Add: Can't Add Entry, \"{}\", to Parent",
                        format_entry_for_error(&entry_str)
                    ));
                }
            };

            match parent {
                Value::Dictionary(dict) => {
                    let key = &path[path.len() - 1];
                    if dict.contains_key(key) {
                        return CommandResult::StderrError(format!(
                            "Add: \"{}\" Entry Already Exists",
                            format_entry_for_error(&entry_str)
                        ));
                    }
                    dict.insert(key.clone(), new_value);
                    state.mutated()
                }
                Value::Array(arr) => {
                    if is_index {
                        let idx: usize = last.parse().unwrap_or(0);
                        let idx = idx.min(arr.len());
                        arr.insert(idx, new_value);
                        state.mutated()
                    } else {
                        CommandResult::StderrError(format!(
                            "Add: Entry, \"{}\", Does Not Exist",
                            format_entry_for_error(&entry_str)
                        ))
                    }
                }
                _ => CommandResult::StderrError(format!(
                    "Add: Can't Add Entry, \"{}\", to Parent",
                    format_entry_for_error(&entry_str)
                )),
            }
        }
    }
}

fn make_value_from_type(type_str: &str, value_str: &str) -> std::result::Result<Option<Value>, String> {
    match type_str.to_lowercase().as_str() {
        "string" => Ok(Some(Value::String(value_str.to_string()))),
        "integer" => {
            let v = value_str.parse::<i64>().unwrap_or(0);
            Ok(Some(Value::Integer(v)))
        }
        "real" => {
            let v = value_str.parse::<f64>().unwrap_or(0.0);
            Ok(Some(Value::Real(v)))
        }
        "bool" => {
            let v = matches!(value_str.to_lowercase().as_str(), "true" | "yes" | "1");
            Ok(Some(Value::Boolean(v)))
        }
        "date" => {
            if !value_str.is_empty() {
                if let Some(d) = parse_date_input(value_str) {
                    return Ok(Some(Value::Date(d)));
                }
            }
            println!("Unrecognized Date Format");
            Ok(None)
        }
        "data" => Ok(Some(Value::Data(value_str.as_bytes().to_vec()))),
        "dict" => Ok(Some(Value::Dictionary(Dictionary::new()))),
        "array" => Ok(Some(Value::Array(Vec::new()))),
        _ => Err(format!("Unrecognized Type: {type_str}")),
    }
}

// --- Copy ---

fn cmd_copy(state: &mut PlistState, args: &str) -> CommandResult {
    let (src_str, rest) = next_token(args);
    let (dst_str, _) = next_token(rest);
    // Empty dst with non-empty src = no-op (copy to same location)
    if dst_str.is_empty() && !src_str.is_empty() {
        return CommandResult::Ok;
    }

    let src_path = parse_entry_path(&src_str);
    let dst_path = parse_entry_path(&dst_str);

    // First, get the source value (clone it)
    let src_value = match resolve_entry(&state.root, &src_path) {
        Ok(v) => v.clone(),
        Err(()) => {
            return CommandResult::StderrError(format!(
                "Copy: Entry, \"{}\", Does Not Exist",
                format_entry_for_error(&src_str)
            ));
        }
    };

    // Check destination doesn't exist, then insert
    if dst_path.is_empty() {
        return CommandResult::StderrError(format!(
            "Copy: \"{}\" Entry Already Exists",
            format_entry_for_error(&dst_str)
        ));
    }

    // Check if dst already exists
    if resolve_entry(&state.root, &dst_path).is_ok() {
        return CommandResult::StderrError(format!(
            "Copy: \"{}\" Entry Already Exists",
            format_entry_for_error(&dst_str)
        ));
    }

    if dst_path.len() == 1 {
        match &mut state.root {
            Value::Dictionary(dict) => {
                dict.insert(dst_path[0].clone(), src_value);
                state.mutated()
            }
            _ => CommandResult::StderrError(format!(
                "Copy: Entry, \"{}\", Does Not Exist",
                format_entry_for_error(&dst_str)
            )),
        }
    } else {
        let parent_path = &dst_path[..dst_path.len() - 1];
        let key = dst_path[dst_path.len() - 1].clone();
        let parent = match ensure_path_mut(&mut state.root, parent_path) {
            Ok(v) => v,
            Err(_) => {
                return CommandResult::StderrError(format!(
                    "Copy: Entry, \"{}\", Does Not Exist",
                    format_entry_for_error(&dst_str)
                ));
            }
        };

        match parent {
            Value::Dictionary(dict) => {
                dict.insert(key, src_value);
                state.mutated()
            }
            _ => CommandResult::StderrError(format!(
                "Copy: Entry, \"{}\", Does Not Exist",
                format_entry_for_error(&dst_str)
            )),
        }
    }
}

// --- Delete ---

fn cmd_delete(state: &mut PlistState, args: &str) -> CommandResult {
    let (entry_str, _) = next_token(args);
    if entry_str.is_empty() {
        println!("Working Container has become Invalid.  Setting to :");
        return CommandResult::Ok;
    }

    let path = parse_entry_path(&entry_str);
    if path.is_empty() {
        return CommandResult::StderrError(format!(
            "Delete: Entry, \"{}\", Does Not Exist",
            format_entry_for_error(&entry_str)
        ));
    }

    let err = || {
        CommandResult::StderrError(format!(
            "Delete: Entry, \"{}\", Does Not Exist",
            format_entry_for_error(&entry_str)
        ))
    };

    let (parent, key) = if path.len() == 1 {
        (&mut state.root, &path[0])
    } else {
        let parent_path = &path[..path.len() - 1];
        let parent = match resolve_entry_mut(&mut state.root, parent_path) {
            Ok(v) => v,
            Err(()) => return err(),
        };
        (parent, &path[path.len() - 1])
    };

    match parent {
        Value::Dictionary(dict) => {
            if dict.remove(key).is_some() {
                state.mutated()
            } else {
                err()
            }
        }
        Value::Array(arr) => {
            if let Ok(idx) = key.parse::<usize>() {
                if idx < arr.len() {
                    arr.remove(idx);
                    state.mutated()
                } else {
                    err()
                }
            } else {
                err()
            }
        }
        _ => err(),
    }
}

// --- Merge ---

fn cmd_merge(state: &mut PlistState, args: &str) -> CommandResult {
    let (file_str, rest) = next_token(args);
    let (entry_str, _) = next_token(rest);

    let merge_path = Path::new(&file_str);
    let source = match Value::from_file(merge_path) {
        Ok(v) => v,
        Err(e) => {
            let io_err = if merge_path.exists() {
                format!("{e}")
            } else {
                "No such file or directory".to_string()
            };
            eprintln!("Error Opening File: {file_str} [{io_err}]");
            return CommandResult::StderrError(format!("Merge: Error Reading File: {file_str}"));
        }
    };

    let target = if entry_str.is_empty() {
        &mut state.root
    } else {
        let path = parse_entry_path(&entry_str);
        if path.is_empty() {
            &mut state.root
        } else {
            match resolve_entry_mut(&mut state.root, &path) {
                Ok(v) => v,
                Err(()) => {
                    return CommandResult::StderrError(format!(
                        "Merge: Entry, \"{}\", Does Not Exist",
                        format_entry_for_error(&entry_str)
                    ));
                }
            }
        }
    };

    // Merge: for dicts, add each key from source to target (if not already present)
    // For arrays, append items
    match (target, &source) {
        (Value::Dictionary(target_dict), Value::Dictionary(source_dict)) => {
            for (key, val) in source_dict.iter() {
                if !target_dict.contains_key(key) {
                    target_dict.insert(key.to_string(), val.clone());
                }
            }
        }
        (Value::Array(target_arr), Value::Array(source_arr)) => {
            for val in source_arr {
                target_arr.push(val.clone());
            }
        }
        (target, source) => {
            let target_type = match target {
                Value::Dictionary(_) => "dict",
                Value::Array(_) => "array",
                _ => "scalar",
            };
            let source_type = match source {
                Value::Dictionary(_) => "dict",
                Value::Array(_) => "array",
                _ => "scalar",
            };
            return CommandResult::StderrError(
                format!("Merge: Can't Add {source_type} Entries to {target_type}")
            );
        }
    }

    state.mutated()
}

// --- Import ---

fn cmd_import(state: &mut PlistState, args: &str) -> CommandResult {
    let (entry_str, rest) = next_token(args);
    let (file_str, _) = next_token(rest);

    let path = parse_entry_path(&entry_str);

    // Check if the target entry is a container (Import can't write to containers)
    let target_is_container = if path.is_empty() {
        matches!(&state.root, Value::Dictionary(_) | Value::Array(_))
    } else {
        matches!(
            resolve_entry(&state.root, &path),
            Ok(Value::Dictionary(_) | Value::Array(_))
        )
    };
    if target_is_container {
        return CommandResult::StderrError(
            "Import: Specified Entry Must Not Be a Container".to_string(),
        );
    }

    let file_path = Path::new(&file_str);
    let data = match std::fs::read(file_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error Opening File: {file_str} [{}]", io_error_message(&e));
            return CommandResult::StderrError(format!("Import: Error Reading File: {file_str}"));
        }
    };

    let new_value = Value::Data(data);

    if path.is_empty() {
        state.root = new_value;
        return state.mutated();
    }

    if let Ok(target) = resolve_entry_mut(&mut state.root, &path) {
        *target = new_value;
        state.mutated()
    } else if path.len() == 1 {
        match &mut state.root {
            Value::Dictionary(dict) => {
                dict.insert(path[0].clone(), new_value);
                state.mutated()
            }
            _ => CommandResult::StderrError(format!(
                "Import: Entry, \"{}\", Does Not Exist",
                format_entry_for_error(&entry_str)
            )),
        }
    } else {
        let parent_path = &path[..path.len() - 1];
        let key = &path[path.len() - 1];
        match resolve_entry_mut(&mut state.root, parent_path) {
            Ok(Value::Dictionary(dict)) => {
                dict.insert(key.clone(), new_value);
                state.mutated()
            }
            _ => CommandResult::StderrError(format!(
                "Import: Entry, \"{}\", Does Not Exist",
                format_entry_for_error(&entry_str)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_test_state() -> PlistState {
        let mut dict = Dictionary::new();
        dict.insert("Name".to_string(), Value::String("Test App".to_string()));
        dict.insert("Version".to_string(), Value::Integer(42));
        dict.insert("Enabled".to_string(), Value::Boolean(true));
        dict.insert("Rating".to_string(), Value::Real(3.14));
        dict.insert(
            "Tags".to_string(),
            Value::Array(vec![
                Value::String("alpha".to_string()),
                Value::String("beta".to_string()),
            ]),
        );
        let mut inner = Dictionary::new();
        inner.insert("Inner".to_string(), Value::String("value".to_string()));
        dict.insert("Nested".to_string(), Value::Dictionary(inner));
        dict.insert("Icon".to_string(), Value::Data(vec![1, 2, 3, 4]));

        PlistState {
            root: Value::Dictionary(dict),
            file_path: PathBuf::from("/tmp/test.plist"),
            xml_output: false,
            dirty: false,
        }
    }

    // --- Entry path parsing ---

    #[test]
    fn test_parse_entry_path_with_colon() {
        assert_eq!(parse_entry_path(":Name"), vec!["Name"]);
    }

    #[test]
    fn test_parse_entry_path_without_colon() {
        assert_eq!(parse_entry_path("Name"), vec!["Name"]);
    }

    #[test]
    fn test_parse_entry_path_nested() {
        assert_eq!(
            parse_entry_path(":Nested:Inner"),
            vec!["Nested", "Inner"]
        );
    }

    #[test]
    fn test_parse_entry_path_empty() {
        assert!(parse_entry_path("").is_empty());
        assert!(parse_entry_path(":").is_empty());
    }

    #[test]
    fn test_parse_entry_path_array_index() {
        assert_eq!(parse_entry_path(":Tags:0"), vec!["Tags", "0"]);
    }

    #[test]
    fn test_parse_entry_path_trailing_colon() {
        assert_eq!(parse_entry_path(":Tags:"), vec!["Tags", ""]);
    }

    // --- Print ---

    #[test]
    fn test_print_string() {
        let state = make_test_state();
        let result = cmd_print(&state, ":Name");
        assert!(matches!(result, CommandResult::Ok));
    }

    #[test]
    fn test_print_nonexistent() {
        let state = make_test_state();
        let result = cmd_print(&state, ":Nonexistent");
        match result {
            CommandResult::StderrError(msg) => {
                assert_eq!(msg, "Print: Entry, \":Nonexistent\", Does Not Exist");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_print_empty_prints_all() {
        let state = make_test_state();
        let result = cmd_print(&state, "");
        assert!(matches!(result, CommandResult::Ok));
    }

    // --- Set ---

    #[test]
    fn test_set_string() {
        let mut state = make_test_state();
        let result = cmd_set(&mut state, ":Name NewName");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert_eq!(d.get("Name").unwrap().as_string().unwrap(), "NewName");
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_set_integer() {
        let mut state = make_test_state();
        let result = cmd_set(&mut state, ":Version 99");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert_eq!(
                    d.get("Version").unwrap().as_signed_integer().unwrap(),
                    99
                );
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_set_bool() {
        let mut state = make_test_state();
        let result = cmd_set(&mut state, ":Enabled false");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert!(!d.get("Enabled").unwrap().as_boolean().unwrap());
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_set_nonexistent() {
        let mut state = make_test_state();
        let result = cmd_set(&mut state, ":Nonexistent foo");
        match result {
            CommandResult::StderrError(msg) => {
                assert_eq!(msg, "Set: Entry, \":Nonexistent\", Does Not Exist");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_set_container_fails() {
        let mut state = make_test_state();
        let result = cmd_set(&mut state, ":Tags foo");
        match result {
            CommandResult::StderrError(msg) => {
                assert_eq!(msg, "Set: Cannot Perform Set On Containers");
            }
            _ => panic!("Expected error"),
        }
    }

    // --- Add ---

    #[test]
    fn test_add_string() {
        let mut state = make_test_state();
        let result = cmd_add(&mut state, ":NewKey string hello");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert_eq!(d.get("NewKey").unwrap().as_string().unwrap(), "hello");
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_add_existing_fails() {
        let mut state = make_test_state();
        let result = cmd_add(&mut state, ":Name string foo");
        match result {
            CommandResult::StderrError(msg) => {
                assert_eq!(msg, "Add: \":Name\" Entry Already Exists");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_add_dict() {
        let mut state = make_test_state();
        let result = cmd_add(&mut state, ":NewDict dict");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert!(d.get("NewDict").unwrap().as_dictionary().is_some());
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_add_array_append() {
        let mut state = make_test_state();
        let result = cmd_add(&mut state, ":Tags: string gamma");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                let arr = d.get("Tags").unwrap().as_array().unwrap();
                assert_eq!(arr.len(), 3);
                assert_eq!(arr[2].as_string().unwrap(), "gamma");
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_add_array_insert_at_index() {
        let mut state = make_test_state();
        let result = cmd_add(&mut state, ":Tags:0 string first");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                let arr = d.get("Tags").unwrap().as_array().unwrap();
                assert_eq!(arr.len(), 3);
                assert_eq!(arr[0].as_string().unwrap(), "first");
                assert_eq!(arr[1].as_string().unwrap(), "alpha");
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_add_integer() {
        let mut state = make_test_state();
        let result = cmd_add(&mut state, ":NewInt integer 99");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert_eq!(
                    d.get("NewInt").unwrap().as_signed_integer().unwrap(),
                    99
                );
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_add_bool() {
        let mut state = make_test_state();
        let result = cmd_add(&mut state, ":NewBool bool true");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert!(d.get("NewBool").unwrap().as_boolean().unwrap());
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_add_real() {
        let mut state = make_test_state();
        let result = cmd_add(&mut state, ":NewReal real 2.718");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                let v = d.get("NewReal").unwrap().as_real().unwrap();
                assert!((v - 2.718).abs() < 1e-6);
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_add_data() {
        let mut state = make_test_state();
        let result = cmd_add(&mut state, ":NewData data AQID");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert_eq!(
                    d.get("NewData").unwrap().as_data().unwrap(),
                    b"AQID"
                );
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_add_unrecognized_type() {
        let mut state = make_test_state();
        let result = cmd_add(&mut state, ":Foo badtype bar");
        match result {
            CommandResult::StdoutError(msg) => {
                assert_eq!(msg, "Unrecognized Type: badtype");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_add_empty_string() {
        let mut state = make_test_state();
        let result = cmd_add(&mut state, ":EmptyStr string");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert_eq!(d.get("EmptyStr").unwrap().as_string().unwrap(), "");
            }
            _ => panic!("Expected dict"),
        }
    }

    // --- Delete ---

    #[test]
    fn test_delete_key() {
        let mut state = make_test_state();
        let result = cmd_delete(&mut state, ":Name");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert!(!d.contains_key("Name"));
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_delete_array_element() {
        let mut state = make_test_state();
        let result = cmd_delete(&mut state, ":Tags:0");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                let arr = d.get("Tags").unwrap().as_array().unwrap();
                assert_eq!(arr.len(), 1);
                assert_eq!(arr[0].as_string().unwrap(), "beta");
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_delete_nonexistent() {
        let mut state = make_test_state();
        let result = cmd_delete(&mut state, ":Nonexistent");
        match result {
            CommandResult::StderrError(msg) => {
                assert_eq!(msg, "Delete: Entry, \":Nonexistent\", Does Not Exist");
            }
            _ => panic!("Expected error"),
        }
    }

    // --- Copy ---

    #[test]
    fn test_copy_entry() {
        let mut state = make_test_state();
        let result = cmd_copy(&mut state, ":Name :NameCopy");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert_eq!(d.get("NameCopy").unwrap().as_string().unwrap(), "Test App");
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_copy_to_existing_fails() {
        let mut state = make_test_state();
        let result = cmd_copy(&mut state, ":Name :Version");
        match result {
            CommandResult::StderrError(msg) => {
                assert_eq!(msg, "Copy: \":Version\" Entry Already Exists");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_copy_nonexistent_source() {
        let mut state = make_test_state();
        let result = cmd_copy(&mut state, ":Nonexistent :Foo");
        match result {
            CommandResult::StderrError(msg) => {
                assert_eq!(msg, "Copy: Entry, \":Nonexistent\", Does Not Exist");
            }
            _ => panic!("Expected error"),
        }
    }

    // --- Clear ---

    #[test]
    fn test_clear_dict() {
        let mut state = make_test_state();
        let result = cmd_clear(&mut state, "dict");
        assert!(matches!(result, CommandResult::Ok));
        assert!(state.root.as_dictionary().unwrap().is_empty());
    }

    #[test]
    fn test_clear_array() {
        let mut state = make_test_state();
        let result = cmd_clear(&mut state, "array");
        assert!(matches!(result, CommandResult::Ok));
        assert!(state.root.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_clear_no_type_defaults_to_dict() {
        let mut state = make_test_state();
        let result = cmd_clear(&mut state, "");
        assert!(matches!(result, CommandResult::Ok));
        assert!(state.root.as_dictionary().unwrap().is_empty());
    }

    // --- Command parsing ---

    #[test]
    fn test_command_case_insensitive() {
        let mut state = make_test_state();
        let result = execute_command(&mut state, "print :Name");
        assert!(matches!(result, CommandResult::Ok));

        let result = execute_command(&mut state, "PRINT :Name");
        assert!(matches!(result, CommandResult::Ok));
    }

    #[test]
    fn test_unknown_command() {
        let mut state = make_test_state();
        let result = execute_command(&mut state, "Foo bar");
        match result {
            CommandResult::StdoutError(msg) => {
                assert_eq!(msg, "Unrecognized Command");
            }
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_empty_command() {
        let mut state = make_test_state();
        let result = execute_command(&mut state, "");
        match result {
            CommandResult::StdoutError(msg) => {
                assert_eq!(msg, "Unrecognized Command");
            }
            _ => panic!("Expected error"),
        }
    }

    // --- Merge ---

    #[test]
    fn test_merge_dict() {
        let mut state = make_test_state();

        // Create a temp file for merge
        let merge_path = std::env::temp_dir().join("test_merge_source.plist");
        let mut source_dict = Dictionary::new();
        source_dict.insert(
            "MergedKey".to_string(),
            Value::String("merged_value".to_string()),
        );
        Value::Dictionary(source_dict)
            .to_file_xml(&merge_path)
            .unwrap();

        let result = cmd_merge(&mut state, merge_path.to_str().unwrap());
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert_eq!(
                    d.get("MergedKey").unwrap().as_string().unwrap(),
                    "merged_value"
                );
            }
            _ => panic!("Expected dict"),
        }

        std::fs::remove_file(&merge_path).ok();
    }

    #[test]
    fn test_merge_nonexistent_file() {
        let mut state = make_test_state();
        let result = cmd_merge(&mut state, "/tmp/no_such_file_merge.plist");
        assert!(matches!(result, CommandResult::StderrError(_)));
    }

    // --- Import ---

    #[test]
    fn test_import_file() {
        let mut state = make_test_state();

        let import_path = std::env::temp_dir().join("test_import_content.txt");
        std::fs::write(&import_path, "hello world\n").unwrap();

        let args = format!(":Imported {}", import_path.to_str().unwrap());
        let result = cmd_import(&mut state, &args);
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert_eq!(
                    d.get("Imported").unwrap().as_data().unwrap(),
                    b"hello world\n"
                );
            }
            _ => panic!("Expected dict"),
        }

        std::fs::remove_file(&import_path).ok();
    }

    #[test]
    fn test_import_nonexistent_file() {
        let mut state = make_test_state();
        let result = cmd_import(&mut state, ":Foo /tmp/no_such_file_import.txt");
        assert!(matches!(result, CommandResult::StderrError(_)));
    }

    // --- Make value from type ---

    #[test]
    fn test_make_value_string() {
        let v = make_value_from_type("string", "hello").unwrap().unwrap();
        assert_eq!(v.as_string().unwrap(), "hello");
    }

    #[test]
    fn test_make_value_integer() {
        let v = make_value_from_type("integer", "42").unwrap().unwrap();
        assert_eq!(v.as_signed_integer().unwrap(), 42);
    }

    #[test]
    fn test_make_value_real() {
        let v = make_value_from_type("real", "3.14").unwrap().unwrap();
        assert!((v.as_real().unwrap() - 3.14).abs() < 1e-6);
    }

    #[test]
    fn test_make_value_bool_true() {
        let v = make_value_from_type("bool", "true").unwrap().unwrap();
        assert!(v.as_boolean().unwrap());
    }

    #[test]
    fn test_make_value_bool_false() {
        let v = make_value_from_type("bool", "false").unwrap().unwrap();
        assert!(!v.as_boolean().unwrap());
    }

    #[test]
    fn test_make_value_dict() {
        let v = make_value_from_type("dict", "").unwrap().unwrap();
        assert!(v.as_dictionary().unwrap().is_empty());
    }

    #[test]
    fn test_make_value_array() {
        let v = make_value_from_type("array", "").unwrap().unwrap();
        assert!(v.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_make_value_data() {
        let v = make_value_from_type("data", "AQID").unwrap().unwrap();
        assert_eq!(v.as_data().unwrap(), b"AQID");
    }

    #[test]
    fn test_make_value_unknown_type() {
        let result = make_value_from_type("badtype", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_type_case_insensitive() {
        assert!(make_value_from_type("String", "hi").is_ok());
        assert!(make_value_from_type("STRING", "hi").is_ok());
        assert!(make_value_from_type("Integer", "1").is_ok());
        assert!(make_value_from_type("BOOL", "true").is_ok());
    }

    // --- Set on nested entries ---

    #[test]
    fn test_set_nested_value() {
        let mut state = make_test_state();
        let result = cmd_set(&mut state, ":Nested:Inner newvalue");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                let nested = d.get("Nested").unwrap().as_dictionary().unwrap();
                assert_eq!(nested.get("Inner").unwrap().as_string().unwrap(), "newvalue");
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_set_array_element() {
        let mut state = make_test_state();
        let result = cmd_set(&mut state, ":Tags:0 newval");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                let arr = d.get("Tags").unwrap().as_array().unwrap();
                assert_eq!(arr[0].as_string().unwrap(), "newval");
            }
            _ => panic!("Expected dict"),
        }
    }

    // --- Delete nested ---

    #[test]
    fn test_delete_nested_key() {
        let mut state = make_test_state();
        let result = cmd_delete(&mut state, ":Nested:Inner");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                let nested = d.get("Nested").unwrap().as_dictionary().unwrap();
                assert!(!nested.contains_key("Inner"));
            }
            _ => panic!("Expected dict"),
        }
    }

    // --- Format entry for error ---

    #[test]
    fn test_format_entry_for_error_with_colon() {
        assert_eq!(format_entry_for_error(":Name"), ":Name");
    }

    #[test]
    fn test_format_entry_for_error_without_colon() {
        assert_eq!(format_entry_for_error("Name"), "Name");
    }

    // --- Date formatting ---

    // --- Coerce value ---

    #[test]
    fn test_coerce_string() {
        let existing = Value::String("old".to_string());
        let new = coerce_value(&existing, "new");
        assert_eq!(new.as_string().unwrap(), "new");
    }

    #[test]
    fn test_coerce_integer() {
        let existing = Value::Integer(0);
        let new = coerce_value(&existing, "42");
        assert_eq!(new.as_signed_integer().unwrap(), 42);
    }

    #[test]
    fn test_coerce_real() {
        let existing = Value::Real(0.0);
        let new = coerce_value(&existing, "3.14");
        assert!((new.as_real().unwrap() - 3.14).abs() < 1e-6);
    }

    #[test]
    fn test_coerce_bool() {
        let existing = Value::Boolean(false);
        let new = coerce_value(&existing, "true");
        assert!(new.as_boolean().unwrap());
    }

    // --- Add to nested dict ---

    #[test]
    fn test_add_to_nested_dict() {
        let mut state = make_test_state();
        let result = cmd_add(&mut state, ":Nested:NewKey string newval");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                let nested = d.get("Nested").unwrap().as_dictionary().unwrap();
                assert_eq!(nested.get("NewKey").unwrap().as_string().unwrap(), "newval");
            }
            _ => panic!("Expected dict"),
        }
    }

    // --- Copy nested ---

    #[test]
    fn test_copy_nested() {
        let mut state = make_test_state();
        let result = cmd_copy(&mut state, ":Nested:Inner :CopiedInner");
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert_eq!(
                    d.get("CopiedInner").unwrap().as_string().unwrap(),
                    "value"
                );
            }
            _ => panic!("Expected dict"),
        }
    }

    // --- Merge into entry ---

    #[test]
    fn test_merge_into_nested() {
        let mut state = make_test_state();

        let merge_path = std::env::temp_dir().join("test_merge_nested.plist");
        let mut source_dict = Dictionary::new();
        source_dict.insert(
            "ExtraKey".to_string(),
            Value::String("extra".to_string()),
        );
        Value::Dictionary(source_dict)
            .to_file_xml(&merge_path)
            .unwrap();

        let args = format!("{} :Nested", merge_path.to_str().unwrap());
        let result = cmd_merge(&mut state, &args);
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                let nested = d.get("Nested").unwrap().as_dictionary().unwrap();
                assert_eq!(
                    nested.get("ExtraKey").unwrap().as_string().unwrap(),
                    "extra"
                );
                // Original key still present
                assert_eq!(nested.get("Inner").unwrap().as_string().unwrap(), "value");
            }
            _ => panic!("Expected dict"),
        }

        std::fs::remove_file(&merge_path).ok();
    }

    // --- Import overwrites existing entry ---

    #[test]
    fn test_import_overwrites_existing() {
        let mut state = make_test_state();

        let import_path = std::env::temp_dir().join("test_import_overwrite.txt");
        std::fs::write(&import_path, "new data").unwrap();

        let args = format!(":Name {}", import_path.to_str().unwrap());
        let result = cmd_import(&mut state, &args);
        assert!(matches!(result, CommandResult::Ok));
        match &state.root {
            Value::Dictionary(d) => {
                assert_eq!(d.get("Name").unwrap().as_data().unwrap(), b"new data");
            }
            _ => panic!("Expected dict"),
        }

        std::fs::remove_file(&import_path).ok();
    }

    // --- execute_command dispatching ---

    #[test]
    fn test_execute_help() {
        let mut state = make_test_state();
        let result = execute_command(&mut state, "Help");
        assert!(matches!(result, CommandResult::Ok));
    }

    #[test]
    fn test_execute_exit() {
        let mut state = make_test_state();
        let result = execute_command(&mut state, "Exit");
        assert!(matches!(result, CommandResult::Exit));
    }

    #[test]
    fn test_execute_save() {
        let mut state = make_test_state();
        let result = execute_command(&mut state, "Save");
        assert!(matches!(result, CommandResult::Save));
    }

    #[test]
    fn test_execute_revert() {
        let mut state = make_test_state();
        let result = execute_command(&mut state, "Revert");
        assert!(matches!(result, CommandResult::Revert));
    }

    // --- split_first_word ---

    #[test]
    fn test_split_first_word() {
        assert_eq!(split_first_word("hello world"), ("hello", "world"));
        assert_eq!(split_first_word("hello"), ("hello", ""));
        assert_eq!(split_first_word("  hello  world  "), ("hello", "world"));
    }
}
