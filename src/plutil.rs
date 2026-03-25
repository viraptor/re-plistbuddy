use crate::value::{AbsoluteTime, Dictionary, Value, CF_EPOCH_OFFSET};
use std::io::Write;
use std::path::Path;

const HELP_TEXT: &str = "\
plutil: [command_option] [other_options] file...
The file '-' means stdin
Running in Swift mode
Command options are (-lint is the default):
 -help                         show this message and exit
 -lint                         check the property list files for syntax errors
 -convert fmt                  rewrite property list files in format
                               fmt is one of: xml1 binary1 json swift objc
                               note: objc can additionally create a header by adding -header
 -insert keypath -type value   insert a value into the property list before writing it out
                               keypath is a key-value coding key path, with one extension:
                               a numerical path component applied to an array will act on the object at that index in the array
                               or insert it into the array if the numerical path component is the last one in the key path
                               type is one of: bool, integer, float, date, string, data, xml, json
                               -bool: YES if passed \"YES\" or \"true\", otherwise NO
                               -integer: any valid 64 bit integer
                               -float: any valid 64 bit float
                               -string: UTF8 encoded string
                               -date: a date in XML property list format, not supported if outputting JSON
                               -data: a base-64 encoded string
                               -xml: an XML property list, useful for inserting compound values
                               -json: a JSON fragment, useful for inserting compound values
                               -dictionary: inserts an empty dictionary, does not use value
                               -array: inserts an empty array, does not use value
                              \x20
                               optionally, -append may be specified if the keypath references an array to append to the
                               end of the array
                               value YES, NO, a number, a date, or a base-64 encoded blob of data
 -replace keypath -type value  same as -insert, but it will overwrite an existing value
 -remove keypath               removes the value at 'keypath' from the property list before writing it out
 -extract keypath fmt          outputs the value at 'keypath' in the property list as a new plist of type 'fmt'
                               fmt is one of: xml1 binary1 json raw
                               an additional \"-expect type\" option can be provided to test that
                               the value at the specified keypath is of the specified \"type\", which
                               can be one of: bool, integer, float, string, date, data, dictionary, array
                              \x20
                               when fmt is raw:\x20
                                   the following is printed to stdout for each value type:
                                       bool: the string \"true\" or \"false\"
                                       integer: the numeric value
                                       float: the numeric value
                                       string: as UTF8-encoded string
                                       date: as RFC3339-encoded string in UTC timezone
                                       data: as base64-encoded string
                                       dictionary: each key on a new line
                                       array: the count of items in the array
                                   by default, the output is to stdout unless -o is specified
 -type keypath                 outputs the type of the value at 'keypath' in the property list
                               can be one of: bool, integer, float, string, date, data, dictionary, array
 -create fmt                   creates an empty plist of the specified format
                               file may be '-' for stdout
 -p                            print property list in a human-readable fashion
                               (not for machine parsing! this 'format' is not stable)
There are some additional optional arguments that apply to the -convert, -insert, -remove, -replace, and -extract verbs:
 -s                            be silent on success
 -o path                       specify alternate file path name for result;
                               the -o option is used with -convert, and is only
                               useful with one file argument (last file overwrites);
                               the path '-' means stdout
 -e extension                  specify alternate extension for converted files
 -r                            if writing JSON, output in human-readable form
 -n                            prevent printing a terminating newline if it is not part of the format, such as with raw
 --                            specifies that all further arguments are file names";

#[derive(Debug, Clone, PartialEq)]
enum Format {
    Xml1,
    Binary1,
    Json,
    Raw,
    Swift,
    Objc,
}

#[derive(Debug)]
enum Command {
    Help,
    Lint,
    Print,
    Convert { format: Format },
    Extract { keypath: String, format: Format, expect: Option<String> },
    Insert { keypath: String, type_name: String, value: Option<String>, append: bool },
    Replace { keypath: String, type_name: String, value: Option<String> },
    Remove { keypath: String },
    Type { keypath: String, expect: Option<String> },
    Create { format: Format },
}

struct Options {
    command: Command,
    files: Vec<String>,
    silent: bool,
    output_path: Option<String>,
    extension: Option<String>,
    readable: bool,
    no_newline: bool,
    header: bool,
}

fn parse_format(s: &str) -> Result<Format, String> {
    match s {
        "xml1" => Ok(Format::Xml1),
        "binary1" => Ok(Format::Binary1),
        "json" => Ok(Format::Json),
        "raw" => Ok(Format::Raw),
        "swift" => Ok(Format::Swift),
        "objc" => Ok(Format::Objc),
        _ => Err(format!("Unknown format specifier: {s}")),
    }
}


fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut command: Option<Command> = None;
    let mut files = Vec::new();
    let mut silent = false;
    let mut output_path = None;
    let mut extension = None;
    let mut readable = false;
    let mut no_newline = false;
    let mut opts_header = false;
    let mut end_of_options = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        if end_of_options || !arg.starts_with('-') || arg == "-" {
            files.push(arg.clone());
            i += 1;
            continue;
        }

        let check_dup = |cmd: &Option<Command>, flag: &str| -> Result<(), String> {
            if cmd.is_some() {
                Err(format!("unrecognized option: {flag}"))
            } else {
                Ok(())
            }
        };

        match arg.as_str() {
            "--" => end_of_options = true,
            "-help" => { check_dup(&command, "-help")?; command = Some(Command::Help); }
            "-lint" => { check_dup(&command, "-lint")?; command = Some(Command::Lint); }
            "-p" => { check_dup(&command, "-p")?; command = Some(Command::Print); }
            "-header" => { opts_header = true; }
            "-s" => silent = true,
            "-r" => readable = true,
            "-n" => no_newline = true,
            "-o" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing path for -o".to_string());
                }
                output_path = Some(args[i].clone());
            }
            "-e" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing extension for -e".to_string());
                }
                extension = Some(args[i].clone());
            }
            "-convert" => {
                check_dup(&command, "-convert")?;
                i += 1;
                if i >= args.len() {
                    return Err("Missing format specifier for command.".to_string());
                }
                let fmt = parse_format(&args[i])?;
                command = Some(Command::Convert { format: fmt });
            }
            "-extract" => {
                check_dup(&command, "-extract")?;
                i += 1;
                if i >= args.len() {
                    return Err("'Extract' requires a key path and a plist format.".to_string());
                }
                let keypath = args[i].clone();
                i += 1;
                if i >= args.len() {
                    return Err("'Extract' requires a key path and a plist format.".to_string());
                }
                let fmt = parse_format(&args[i])?;
                let mut expect = None;
                if i + 1 < args.len() && args[i + 1] == "-expect" {
                    i += 2;
                    if i >= args.len() {
                        return Err("Missing type for -expect".to_string());
                    }
                    expect = Some(args[i].clone());
                }
                command = Some(Command::Extract { keypath, format: fmt, expect });
            }
            "-type" => {
                check_dup(&command, "-type")?;
                i += 1;
                if i >= args.len() {
                    return Err("'Extract' requires a key path and a plist format.".to_string());
                }
                let keypath = args[i].clone();
                let mut expect = None;
                if i + 1 < args.len() && args[i + 1] == "-expect" {
                    i += 2;
                    if i >= args.len() {
                        return Err("Missing type for -expect".to_string());
                    }
                    expect = Some(args[i].clone());
                }
                command = Some(Command::Type { keypath, expect });
            }
            "-insert" => {
                check_dup(&command, "-insert")?;
                i += 1;
                if i >= args.len() {
                    return Err("'Insert' and 'Replace' require a key path, a type, and a value.".to_string());
                }
                let keypath = args[i].clone();
                i += 1;
                if i >= args.len() {
                    return Err("'Insert' and 'Replace' require a key path, a type, and a value.".to_string());
                }
                let type_name = args[i].trim_start_matches('-').to_string();
                let mut value = None;
                let mut append = false;
                if type_name != "dictionary" && type_name != "array" {
                    if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        i += 1;
                        value = Some(args[i].clone());
                    } else if i + 1 < args.len() && args[i + 1] != "-append" && args[i + 1] != "-s" && args[i + 1] != "-o" {
                        i += 1;
                        value = Some(args[i].clone());
                    }
                }
                if i + 1 < args.len() && args[i + 1] == "-append" {
                    i += 1;
                    append = true;
                }
                command = Some(Command::Insert { keypath, type_name, value, append });
            }
            "-replace" => {
                check_dup(&command, "-replace")?;
                i += 1;
                if i >= args.len() {
                    return Err("'Insert' and 'Replace' require a key path, a type, and a value.".to_string());
                }
                let keypath = args[i].clone();
                i += 1;
                if i >= args.len() {
                    return Err("'Insert' and 'Replace' require a key path, a type, and a value.".to_string());
                }
                let type_name = args[i].trim_start_matches('-').to_string();
                let mut value = None;
                if type_name != "dictionary" && type_name != "array" {
                    if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        i += 1;
                        value = Some(args[i].clone());
                    }
                }
                command = Some(Command::Replace { keypath, type_name, value });
            }
            "-remove" => {
                check_dup(&command, "-remove")?;
                i += 1;
                if i >= args.len() {
                    return Err("'Remove' requires a key path.".to_string());
                }
                command = Some(Command::Remove { keypath: args[i].clone() });
            }
            "-create" => {
                check_dup(&command, "-create")?;
                i += 1;
                if i >= args.len() {
                    return Err("'Create' requires a plist format.".to_string());
                }
                let fmt = parse_format(&args[i])?;
                command = Some(Command::Create { format: fmt });
            }
            other => {
                return Err(format!("unrecognized option: {other}"));
            }
        }
        i += 1;
    }

    if command.is_none() {
        command = Some(Command::Lint);
    }

    Ok(Options {
        command: command.unwrap(),
        files,
        silent,
        output_path,
        extension,
        readable,
        no_newline,
        header: opts_header,
    })
}

pub fn run(args: &[String]) -> anyhow::Result<u8> {
    let opts = match parse_args(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return Ok(1);
        }
    };

    if matches!(opts.command, Command::Help) {
        println!("{HELP_TEXT}");
        return Ok(0);
    }

    if opts.files.is_empty() && !matches!(opts.command, Command::Help) {
        eprintln!("No files specified.");
        return Ok(1);
    }

    let mut any_failed = false;

    for file_arg in &opts.files {
        let result = process_file(file_arg, &opts);
        if let Err(e) = result {
            let msg = format!("{e}");
            if !msg.is_empty() {
                eprintln!("{file_arg}: {e}");
            }
            any_failed = true;
        }
    }

    Ok(if any_failed { 1 } else { 0 })
}

fn read_input(file_arg: &str) -> anyhow::Result<Value> {
    if file_arg == "-" {
        let mut buf = Vec::new();
        std::io::stdin().lock().read_to_end(&mut buf)?;
        let tmp = std::env::temp_dir().join("plutil_stdin.plist");
        std::fs::write(&tmp, &buf)?;
        let val = Value::from_file(&tmp)?;
        std::fs::remove_file(&tmp).ok();
        Ok(val)
    } else {
        let path = Path::new(file_arg);
        if !path.exists() {
            let filename = path.file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| file_arg.to_string());
            anyhow::bail!(
                "(The file \u{201c}{filename}\u{201d} couldn\u{2019}t be opened because there is no such file.)"
            );
        }
        // Try CF plist reader first, fall back to JSON parser
        match Value::from_file(path) {
            Ok(v) => Ok(v),
            Err(cf_err) => {
                let content = std::fs::read_to_string(path).unwrap_or_default();
                let trimmed = content.trim();
                if trimmed.is_empty() {
                    // Empty file = empty dict (matches Apple behavior)
                    return Ok(Value::Dictionary(Dictionary::new()));
                }
                if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    match parse_json_value(trimmed) {
                        Ok(v) => Ok(v),
                        Err(_) => Err(cf_err),
                    }
                } else {
                    Err(cf_err)
                }
            }
        }
    }
}

use std::io::Read;

fn process_file(file_arg: &str, opts: &Options) -> anyhow::Result<()> {
    match &opts.command {
        Command::Help => unreachable!(),
        Command::Lint => {
            // Lint uses strict CF plist reader only (no JSON fallback)
            let result = if file_arg == "-" {
                let mut buf = Vec::new();
                std::io::stdin().lock().read_to_end(&mut buf)?;
                let tmp = std::env::temp_dir().join("plutil_lint_stdin.plist");
                std::fs::write(&tmp, &buf)?;
                let r = Value::from_file(&tmp);
                std::fs::remove_file(&tmp).ok();
                r
            } else {
                let path = Path::new(file_arg);
                if !path.exists() {
                    let filename = path.file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| file_arg.to_string());
                    Err(anyhow::anyhow!(
                        "(The file \u{201c}{filename}\u{201d} couldn\u{2019}t be opened because there is no such file.)"
                    ))
                } else {
                    let content = std::fs::read(path)?;
                    if content.is_empty() || content.iter().all(|&b| b == b'\n' || b == b' ') {
                        Ok(Value::Dictionary(Dictionary::new()))
                    } else {
                        Value::from_file(path)
                    }
                }
            };
            match result {
                Ok(_) => {
                    if !opts.silent {
                        let name = if file_arg == "-" { "<stdin>" } else { file_arg };
                        println!("{name}: OK");
                    }
                }
                Err(e) => {
                    let name = if file_arg == "-" { "<stdin>" } else { file_arg };
                    let msg = format!("{e}");
                    // CF parse errors need parens, but missing-file errors already have them
                    if msg.starts_with('(') {
                        eprintln!("{name}: {msg}");
                    } else {
                        eprintln!("{name}: ({msg})");
                    }
                    return Err(anyhow::anyhow!(""));
                }
            }
        }
        Command::Print => {
            let val = read_input(file_arg)?;
            pretty_print(&val, 0);
        }
        Command::Convert { format } => {
            let val = read_input(file_arg)?;
            let out_path = resolve_output_path(file_arg, opts);
            if *format == Format::Objc {
                if value_contains_objc_invalid(&val) {
                    anyhow::bail!("Input contains an object that cannot be represented in Obj-C literal syntax");
                }
                if out_path == "-" {
                    let objc = value_to_objc(&val, file_arg, false);
                    print!("{objc}");
                } else if opts.header {
                    let var = var_name_from_path(file_arg);
                    let h_path = Path::new(&out_path).with_extension("h");
                    let h_content = value_to_objc_header(&val, file_arg, &var);
                    std::fs::write(&h_path, h_content.as_bytes())?;
                    let m_path = Path::new(&out_path).with_extension("m");
                    let m_content = value_to_objc(&val, file_arg, true);
                    std::fs::write(&m_path, m_content.as_bytes())?;
                } else {
                    let objc = value_to_objc(&val, file_arg, false);
                    std::fs::write(&out_path, objc.as_bytes())?;
                }
            } else {
                write_value_as_format(&val, &out_path, format, opts.readable, file_arg)?;
            }
        }
        Command::Extract { keypath, format, expect } => {
            let val = read_input(file_arg)?;
            let extracted = resolve_keypath(&val, keypath)
                .ok_or_else(|| anyhow::anyhow!(
                    "Could not extract value, error: No value at that key path or invalid key path: {keypath}"
                ))?;
            if let Some(expected_type) = expect {
                check_type(extracted, expected_type, keypath)?;
            }
            let out_path = resolve_output_path(file_arg, opts);
            if *format == Format::Raw {
                let raw = format_raw(extracted);
                write_output(&out_path, raw.as_bytes(), opts.no_newline)?;
            } else {
                if *format == Format::Json && is_json_invalid(extracted) {
                    anyhow::bail!("Invalid object in plist for JSON format");
                }
                write_value_as_format(extracted, &out_path, format, opts.readable, file_arg)?;
            }
            if !opts.silent {
                // silent by default on success
            }
        }
        Command::Type { keypath, expect } => {
            let val = read_input(file_arg)?;
            let target = resolve_keypath(&val, keypath)
                .ok_or_else(|| anyhow::anyhow!(
                    "Could not extract value, error: No value at that key path or invalid key path: {keypath}"
                ))?;
            if let Some(expected_type) = expect {
                check_type(target, expected_type, keypath)?;
            }
            println!("{}", type_name(target));
        }
        Command::Insert { keypath, type_name, value, append } => {
            let mut val = read_input(file_arg)?;
            let new_value = make_value(type_name, value.as_deref())?;
            insert_at_keypath(&mut val, keypath, new_value, *append)?;
            let out_path = resolve_output_path(file_arg, opts);
            write_value_as_format(&val, &out_path, &Format::Xml1, false, file_arg)?;
        }
        Command::Replace { keypath, type_name, value } => {
            let mut val = read_input(file_arg)?;
            let new_value = make_value(type_name, value.as_deref())?;
            replace_at_keypath(&mut val, keypath, new_value)?;
            let out_path = resolve_output_path(file_arg, opts);
            write_value_as_format(&val, &out_path, &Format::Xml1, false, file_arg)?;
        }
        Command::Remove { keypath } => {
            let mut val = read_input(file_arg)?;
            remove_at_keypath(&mut val, keypath)?;
            let out_path = resolve_output_path(file_arg, opts);
            write_value_as_format(&val, &out_path, &Format::Xml1, false, file_arg)?;
        }
        Command::Create { format } => {
            let val = Value::Dictionary(Dictionary::new());
            let out_path = if file_arg == "-" {
                "-".to_string()
            } else {
                file_arg.to_string()
            };
            write_value_as_format(&val, &out_path, format, opts.readable, file_arg)?;
        }
    }
    Ok(())
}

fn resolve_output_path(file_arg: &str, opts: &Options) -> String {
    if let Some(ref o) = opts.output_path {
        o.clone()
    } else if let Some(ref ext) = opts.extension {
        let p = Path::new(file_arg);
        p.with_extension(ext).to_string_lossy().to_string()
    } else {
        file_arg.to_string()
    }
}

fn write_output(path: &str, data: &[u8], no_newline: bool) -> anyhow::Result<()> {
    if path == "-" {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        out.write_all(data)?;
        if !no_newline {
            out.write_all(b"\n")?;
        }
        out.flush()?;
    } else {
        // Raw output to file never adds a newline (matching Apple's behavior)
        std::fs::write(path, data)?;
    }
    Ok(())
}

fn write_value_as_format(value: &Value, path: &str, format: &Format, readable: bool, source_name: &str) -> anyhow::Result<()> {
    match format {
        Format::Xml1 => {
            if path == "-" {
                let bytes = value.to_xml_bytes()?;
                std::io::stdout().write_all(&bytes)?;
            } else {
                value.to_file_xml(Path::new(path))?;
            }
        }
        Format::Binary1 => {
            if path == "-" {
                // Write to temp, then copy to stdout
                let tmp = std::env::temp_dir().join("plutil_bin_tmp.plist");
                value.to_file_binary(&tmp)?;
                let data = std::fs::read(&tmp)?;
                std::io::stdout().write_all(&data)?;
                std::fs::remove_file(&tmp).ok();
            } else {
                value.to_file_binary(Path::new(path))?;
            }
        }
        Format::Json => {
            if is_json_invalid(value) {
                anyhow::bail!("Invalid object in plist for JSON format");
            }
            let json = if readable {
                value_to_json_readable(value, 0)
            } else {
                value_to_json_compact(value)
            };
            if path == "-" {
                print!("{json}");
            } else {
                std::fs::write(path, json.as_bytes())?;
            }
        }
        Format::Swift => {
            let swift = value_to_swift(value, source_name);
            if path == "-" {
                print!("{swift}");
            } else {
                std::fs::write(path, swift.as_bytes())?;
            }
        }
        Format::Objc => {
            anyhow::bail!("use process_file for objc format");
        }
        Format::Raw => {
            anyhow::bail!("raw format is only valid for -extract");
        }
    }
    Ok(())
}

fn resolve_keypath<'a>(root: &'a Value, keypath: &str) -> Option<&'a Value> {
    if keypath.is_empty() {
        return None;
    }
    let components: Vec<&str> = keypath.split('.').collect();
    let mut current = root;
    for component in &components {
        current = match current {
            Value::Dictionary(dict) => dict.get(component)?,
            Value::Array(arr) => {
                let idx: usize = component.parse().ok()?;
                arr.get(idx)?
            }
            _ => return None,
        };
    }
    Some(current)
}

fn resolve_keypath_mut<'a>(root: &'a mut Value, keypath: &str) -> Option<&'a mut Value> {
    if keypath.is_empty() {
        return Some(root);
    }
    let components: Vec<&str> = keypath.split('.').collect();
    let mut current = root;
    for component in &components {
        current = match current {
            Value::Dictionary(dict) => dict.get_mut(component)?,
            Value::Array(arr) => {
                let idx: usize = component.parse().ok()?;
                arr.get_mut(idx)?
            }
            _ => return None,
        };
    }
    Some(current)
}

fn resolve_parent_and_key<'a>(root: &'a mut Value, keypath: &str) -> Option<(&'a mut Value, String)> {
    let components: Vec<&str> = keypath.split('.').collect();
    if components.is_empty() {
        return None;
    }
    if components.len() == 1 {
        return Some((root, components[0].to_string()));
    }
    let parent_path = components[..components.len() - 1].join(".");
    let key = components.last()?.to_string();
    let parent = resolve_keypath_mut(root, &parent_path)?;
    Some((parent, key))
}

fn insert_at_keypath(root: &mut Value, keypath: &str, new_value: Value, append: bool) -> anyhow::Result<()> {
    if append {
        let target = resolve_keypath_mut(root, keypath)
            .ok_or_else(|| anyhow::anyhow!("Key path not found {keypath}"))?;
        match target {
            Value::Array(arr) => {
                arr.push(new_value);
                return Ok(());
            }
            _ => anyhow::bail!("Appending to a non-array at key path {keypath}"),
        }
    }

    let (parent, key) = resolve_parent_and_key(root, keypath)
        .ok_or_else(|| anyhow::anyhow!("Key path not found {keypath}"))?;

    match parent {
        Value::Dictionary(dict) => {
            if dict.contains_key(&key) {
                anyhow::bail!("Value already exists at key path {keypath}");
            }
            dict.insert(key, new_value);
        }
        Value::Array(arr) => {
            let idx: usize = key.parse()
                .map_err(|_| anyhow::anyhow!("Invalid array index: {key}"))?;
            let idx = idx.min(arr.len());
            arr.insert(idx, new_value);
        }
        _ => anyhow::bail!("Cannot insert into non-container at keypath"),
    }
    Ok(())
}

fn replace_at_keypath(root: &mut Value, keypath: &str, new_value: Value) -> anyhow::Result<()> {
    let target = resolve_keypath_mut(root, keypath)
        .ok_or_else(|| anyhow::anyhow!("No value at that key path or invalid key path: {keypath}"))?;
    *target = new_value;
    Ok(())
}

fn remove_at_keypath(root: &mut Value, keypath: &str) -> anyhow::Result<()> {
    let (parent, key) = resolve_parent_and_key(root, keypath)
        .ok_or_else(|| anyhow::anyhow!("No value at that key path or invalid key path: {keypath}"))?;

    match parent {
        Value::Dictionary(dict) => {
            dict.remove(&key)
                .ok_or_else(|| anyhow::anyhow!("No value to remove at key path {keypath}"))?;
        }
        Value::Array(arr) => {
            let idx: usize = key.parse()
                .map_err(|_| anyhow::anyhow!("Invalid array index: {key}"))?;
            if idx >= arr.len() {
                anyhow::bail!("Array index out of bounds: {idx}");
            }
            arr.remove(idx);
        }
        _ => anyhow::bail!("Cannot remove from non-container"),
    }
    Ok(())
}

fn make_value(type_name: &str, value_str: Option<&str>) -> anyhow::Result<Value> {
    match type_name {
        "bool" => {
            let s = value_str.unwrap_or("false");
            let v = s == "YES" || s == "true";
            Ok(Value::Boolean(v))
        }
        "integer" => {
            let s = value_str.unwrap_or("0");
            let v: i64 = s.parse().map_err(|_| anyhow::anyhow!("Invalid integer: {s}"))?;
            Ok(Value::Integer(v))
        }
        "float" => {
            let s = value_str.unwrap_or("0");
            let v: f64 = s.parse().map_err(|_| anyhow::anyhow!("Invalid float: {s}"))?;
            Ok(Value::Real(v))
        }
        "string" => {
            Ok(Value::String(value_str.unwrap_or("").to_string()))
        }
        "date" => {
            let s = value_str.ok_or_else(|| anyhow::anyhow!("Missing date value"))?;
            // Parse ISO 8601 date
            let abs = parse_iso_date(s)
                .ok_or_else(|| anyhow::anyhow!("Invalid date format: {s}"))?;
            Ok(Value::Date(abs))
        }
        "data" => {
            let s = value_str.unwrap_or("");
            let bytes = base64_decode(s)
                .ok_or_else(|| anyhow::anyhow!("Invalid base64 data"))?;
            Ok(Value::Data(bytes))
        }
        "dictionary" => Ok(Value::Dictionary(Dictionary::new())),
        "array" => Ok(Value::Array(Vec::new())),
        "xml" => {
            let s = value_str.ok_or_else(|| anyhow::anyhow!("Missing XML value"))?;
            // Parse XML plist fragment
            let tmp = std::env::temp_dir().join("plutil_xml_tmp.plist");
            std::fs::write(&tmp, s)?;
            let val = Value::from_file(&tmp)?;
            std::fs::remove_file(&tmp).ok();
            Ok(val)
        }
        "json" => {
            let s = value_str.ok_or_else(|| anyhow::anyhow!("Missing JSON value"))?;
            parse_json_value(s).map_err(|e| anyhow::anyhow!("Invalid JSON: {e}"))
        }
        _ => anyhow::bail!("Unknown type: {type_name}"),
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "string",
        Value::Integer(_) => "integer",
        Value::Real(_) => "float",
        Value::Boolean(_) => "bool",
        Value::Date(_) => "date",
        Value::Data(_) => "data",
        Value::Array(_) => "array",
        Value::Dictionary(_) => "dictionary",
    }
}

fn check_type(value: &Value, expected: &str, keypath: &str) -> anyhow::Result<()> {
    let actual = type_name(value);
    if actual != expected {
        anyhow::bail!("Value at [{keypath}] expected to be {expected} but is {actual}");
    }
    Ok(())
}

fn format_raw(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Integer(i) => i.to_string(),
        Value::Real(f) => {
            crate::cf::format_double_6f(*f)
        }
        Value::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Date(abs) => format_iso_date(*abs),
        Value::Data(bytes) => base64_encode(bytes),
        Value::Array(arr) => arr.len().to_string(),
        Value::Dictionary(dict) => {
            let mut keys: Vec<&str> = dict.iter().map(|(k, _)| k).collect();
            keys.sort();
            keys.join("\n")
        }
    }
}

fn format_iso_date(abs: AbsoluteTime) -> String {
    let unix = (abs + CF_EPOCH_OFFSET) as i64;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::gmtime_r(&unix, &mut tm) };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec
    )
}

fn parse_iso_date(s: &str) -> Option<AbsoluteTime> {
    // Parse "YYYY-MM-DDTHH:MM:SSZ"
    if s.len() < 20 {
        return None;
    }
    let year: i32 = s[0..4].parse().ok()?;
    let month: i32 = s[5..7].parse().ok()?;
    let day: i32 = s[8..10].parse().ok()?;
    let hour: i32 = s[11..13].parse().ok()?;
    let min: i32 = s[14..16].parse().ok()?;
    let sec: i32 = s[17..19].parse().ok()?;

    let mut tm = libc::tm {
        tm_sec: sec,
        tm_min: min,
        tm_hour: hour,
        tm_mday: day,
        tm_mon: month - 1,
        tm_year: year - 1900,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null_mut(),
    };
    let time_t = unsafe { libc::timegm(&mut tm) };
    if time_t == -1 {
        return None;
    }
    Some(time_t as f64 - CF_EPOCH_OFFSET)
}

// Simple base64 implementation to avoid adding a dependency
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    fn char_val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let s: Vec<u8> = s.bytes().filter(|&b| b != b'\n' && b != b'\r' && b != b' ').collect();
    if s.len() % 4 != 0 {
        return None;
    }
    let mut result = Vec::with_capacity(s.len() / 4 * 3);
    for chunk in s.chunks(4) {
        let a = char_val(chunk[0])?;
        let b = char_val(chunk[1])?;
        result.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            let c = char_val(chunk[2])?;
            result.push(((b & 0xf) << 4) | (c >> 2));
            if chunk[3] != b'=' {
                let d = char_val(chunk[3])?;
                result.push(((c & 0x3) << 6) | d);
            }
        }
    }
    Some(result)
}

fn is_json_invalid_nested(value: &Value) -> bool {
    match value {
        Value::Date(_) | Value::Data(_) => true,
        Value::Array(arr) => arr.iter().any(is_json_invalid_nested),
        Value::Dictionary(dict) => dict.iter().any(|(_, v)| is_json_invalid_nested(v)),
        _ => false,
    }
}

// Both -convert json and -extract json require a container root without date/data
fn is_json_invalid(value: &Value) -> bool {
    match value {
        Value::Array(arr) => arr.iter().any(is_json_invalid_nested),
        Value::Dictionary(dict) => dict.iter().any(|(_, v)| is_json_invalid_nested(v)),
        _ => true,
    }
}

fn parse_json_value(s: &str) -> Result<Value, String> {
    let s = s.trim();
    let (val, rest) = parse_json_inner(s)?;
    let rest = rest.trim();
    if !rest.is_empty() {
        return Err(format!("trailing content: {rest}"));
    }
    Ok(val)
}

fn parse_json_inner(s: &str) -> Result<(Value, &str), String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("unexpected end of input".to_string());
    }
    match s.as_bytes()[0] {
        b'"' => {
            let (string, rest) = parse_json_string(s)?;
            Ok((Value::String(string), rest))
        }
        b'{' => parse_json_object(&s[1..]),
        b'[' => parse_json_array(&s[1..]),
        b't' if s.starts_with("true") => Ok((Value::Boolean(true), &s[4..])),
        b'f' if s.starts_with("false") => Ok((Value::Boolean(false), &s[5..])),
        b'n' if s.starts_with("null") => Ok((Value::String(String::new()), &s[4..])),
        b'-' | b'0'..=b'9' => parse_json_number(s),
        c => Err(format!("unexpected character: {}", c as char)),
    }
}

fn parse_json_string(s: &str) -> Result<(String, &str), String> {
    if !s.starts_with('"') {
        return Err("expected string".to_string());
    }
    let s = &s[1..];
    let mut result = String::new();
    let mut chars = s.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Ok((result, &s[i + 1..])),
            '\\' => {
                match chars.next() {
                    Some((_, '"')) => result.push('"'),
                    Some((_, '\\')) => result.push('\\'),
                    Some((_, '/')) => result.push('/'),
                    Some((_, 'n')) => result.push('\n'),
                    Some((_, 'r')) => result.push('\r'),
                    Some((_, 't')) => result.push('\t'),
                    Some((_, 'u')) => {
                        let hex: String = chars.by_ref().take(4).map(|(_, c)| c).collect();
                        let code = u32::from_str_radix(&hex, 16)
                            .map_err(|_| "invalid unicode escape".to_string())?;
                        if let Some(ch) = char::from_u32(code) {
                            result.push(ch);
                        }
                    }
                    _ => return Err("invalid escape".to_string()),
                }
            }
            c => result.push(c),
        }
    }
    Err("unterminated string".to_string())
}

fn parse_json_number(s: &str) -> Result<(Value, &str), String> {
    let mut end = 0;
    let bytes = s.as_bytes();
    if end < bytes.len() && bytes[end] == b'-' { end += 1; }
    while end < bytes.len() && bytes[end].is_ascii_digit() { end += 1; }
    let is_float = end < bytes.len() && (bytes[end] == b'.' || bytes[end] == b'e' || bytes[end] == b'E');
    if is_float {
        if end < bytes.len() && bytes[end] == b'.' { end += 1; }
        while end < bytes.len() && bytes[end].is_ascii_digit() { end += 1; }
        if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
            end += 1;
            if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') { end += 1; }
            while end < bytes.len() && bytes[end].is_ascii_digit() { end += 1; }
        }
        let num_str = &s[..end];
        let v: f64 = num_str.parse().map_err(|_| format!("invalid number: {num_str}"))?;
        Ok((Value::Real(v), &s[end..]))
    } else {
        let num_str = &s[..end];
        let v: i64 = num_str.parse().map_err(|_| format!("invalid number: {num_str}"))?;
        Ok((Value::Integer(v), &s[end..]))
    }
}

fn parse_json_object(s: &str) -> Result<(Value, &str), String> {
    let mut s = s.trim();
    let mut dict = Dictionary::new();
    if s.starts_with('}') {
        return Ok((Value::Dictionary(dict), &s[1..]));
    }
    loop {
        let (key, rest) = parse_json_string(s.trim())?;
        let rest = rest.trim();
        if !rest.starts_with(':') {
            return Err("expected ':' in object".to_string());
        }
        let (val, rest) = parse_json_inner(&rest[1..])?;
        dict.insert(key, val);
        s = rest.trim();
        if s.starts_with('}') {
            return Ok((Value::Dictionary(dict), &s[1..]));
        }
        if !s.starts_with(',') {
            return Err("expected ',' or '}' in object".to_string());
        }
        s = &s[1..];
    }
}

fn parse_json_array(s: &str) -> Result<(Value, &str), String> {
    let mut s = s.trim();
    let mut arr = Vec::new();
    if s.starts_with(']') {
        return Ok((Value::Array(arr), &s[1..]));
    }
    loop {
        let (val, rest) = parse_json_inner(s.trim())?;
        arr.push(val);
        s = rest.trim();
        if s.starts_with(']') {
            return Ok((Value::Array(arr), &s[1..]));
        }
        if !s.starts_with(',') {
            return Err("expected ',' or ']' in array".to_string());
        }
        s = &s[1..];
    }
}

fn json_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    result.push('"');
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

fn value_to_json_compact(value: &Value) -> String {
    match value {
        Value::String(s) => json_escape(s),
        Value::Integer(i) => i.to_string(),
        Value::Real(f) => format_json_real(*f),
        Value::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Date(_) | Value::Data(_) => {
            // JSON doesn't support date/data - this shouldn't be called
            "null".to_string()
        }
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(value_to_json_compact).collect();
            format!("[{}]", items.join(","))
        }
        Value::Dictionary(dict) => {
            let items: Vec<String> = dict.iter()
                .map(|(k, v)| format!("{}:{}", json_escape(k), value_to_json_compact(v)))
                .collect();
            format!("{{{}}}", items.join(","))
        }
    }
}

fn value_to_json_readable(value: &Value, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let pad_inner = "  ".repeat(indent + 1);
    match value {
        Value::String(s) => json_escape(s),
        Value::Integer(i) => i.to_string(),
        Value::Real(f) => format_json_real(*f),
        Value::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Date(_) | Value::Data(_) => "null".to_string(),
        Value::Array(arr) => {
            if arr.is_empty() {
                return "[\n\n]".to_string();
            }
            let items: Vec<String> = arr.iter()
                .map(|v| format!("{pad_inner}{}", value_to_json_readable(v, indent + 1)))
                .collect();
            format!("[\n{}\n{pad}]", items.join(",\n"))
        }
        Value::Dictionary(dict) => {
            if dict.is_empty() {
                return "{\n\n}".to_string();
            }
            // Sort keys for readable output
            let mut entries: Vec<(&str, &Value)> = dict.iter().collect();
            entries.sort_by(|(a, _), (b, _)| natural_cmp(a, b));
            let items: Vec<String> = entries.iter()
                .map(|(k, v)| format!("{pad_inner}{} : {}", json_escape(k), value_to_json_readable(v, indent + 1)))
                .collect();
            format!("{{\n{}\n{pad}}}", items.join(",\n"))
        }
    }
}

fn format_json_real(f: f64) -> String {
    // Match Apple's JSON real formatting
    let mut buf = [0u8; 64];
    let fmt = b"%.17g\0";
    let len = unsafe {
        libc::snprintf(
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            fmt.as_ptr() as *const libc::c_char,
            f,
        )
    };
    let len = (len as usize).min(buf.len() - 1);
    let s = std::str::from_utf8(&buf[..len]).unwrap_or("0");
    // Apple appends extra zeros after the decimal point - use their format
    s.to_string()
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();

    loop {
        match (ai.peek(), bi.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let an: String = ai.by_ref().take_while(|c| c.is_ascii_digit()).collect();
                    let bn: String = bi.by_ref().take_while(|c| c.is_ascii_digit()).collect();
                    let av: u64 = an.parse().unwrap_or(0);
                    let bv: u64 = bn.parse().unwrap_or(0);
                    match av.cmp(&bv) {
                        std::cmp::Ordering::Equal => {}
                        other => return other,
                    }
                } else {
                    let al = ac.to_lowercase().next().unwrap_or(ac);
                    let bl = bc.to_lowercase().next().unwrap_or(bc);
                    match al.cmp(&bl) {
                        std::cmp::Ordering::Equal => {
                            ai.next();
                            bi.next();
                        }
                        other => return other,
                    }
                }
            }
        }
    }
}

fn var_name_from_path(path: &str) -> String {
    let name = Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "plist".to_string());
    name.replace('-', "_").replace(' ', "_")
}

fn swift_type_tag(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "string",
        Value::Integer(_) => "integer",
        Value::Real(_) => "real",
        Value::Boolean(_) => "bool",
        Value::Date(_) => "date",
        Value::Data(_) => "data",
        Value::Array(_) => "array",
        Value::Dictionary(_) => "dict",
    }
}

fn array_needs_any_annotation(arr: &[Value]) -> bool {
    let mut tags = arr.iter().map(swift_type_tag);
    match tags.next() {
        None => false,
        Some(first) => tags.any(|t| t != first),
    }
}

fn dict_needs_any_annotation(dict: &Dictionary) -> bool {
    let mut tags = dict.iter().map(|(_, v)| swift_type_tag(v));
    match tags.next() {
        None => false,
        Some(first) => tags.any(|t| t != first),
    }
}

fn value_to_swift(value: &Value, path: &str) -> String {
    let filename = Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "plist".to_string());
    let var = var_name_from_path(path);

    let mut out = String::new();
    out.push_str(&format!("/// Generated from {filename}\n"));

    match value {
        Value::Dictionary(dict) => {
            if dict_needs_any_annotation(dict) {
                out.push_str(&format!("let {var} : [String : Any] = [\n"));
            } else {
                out.push_str(&format!("let {var} = [\n"));
            }
            let mut entries: Vec<(&str, &Value)> = dict.iter().collect();
            entries.sort_by(|(a, _), (b, _)| natural_cmp(a, b));
            for (key, val) in &entries {
                out.push_str(&format!("    {} : ", swift_string_literal(key)));
                swift_value(&mut out, val, 1);
                out.push_str(",\n");
            }
            out.push_str("]\n");
        }
        Value::Array(arr) => {
            if array_needs_any_annotation(arr) {
                out.push_str(&format!("let {var} : [Any] = [\n"));
            } else {
                out.push_str(&format!("let {var} = [\n"));
            }
            for item in arr {
                out.push_str("    ");
                swift_value(&mut out, item, 1);
                out.push_str(",\n");
            }
            out.push_str("]\n");
        }
        _ => {
            out.push_str(&format!("let {var} = "));
            swift_value(&mut out, value, 0);
            out.push('\n');
        }
    }
    out
}

fn swift_value(out: &mut String, value: &Value, indent: usize) {
    let pad = "    ".repeat(indent);
    match value {
        Value::String(s) => out.push_str(&swift_string_literal(s)),
        Value::Integer(i) => out.push_str(&i.to_string()),
        Value::Real(f) => {
            let mut buf = [0u8; 64];
            let fmt = b"%.6f\0";
            let len = unsafe {
                libc::snprintf(
                    buf.as_mut_ptr() as *mut libc::c_char,
                    buf.len(),
                    fmt.as_ptr() as *const libc::c_char,
                    *f,
                )
            };
            let len = (len as usize).min(buf.len() - 1);
            out.push_str(std::str::from_utf8(&buf[..len]).unwrap_or("0"));
        }
        Value::Boolean(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Date(abs) => {
            out.push_str(&format!("Date(timeIntervalSinceReferenceDate: {:.1})", abs));
        }
        Value::Data(bytes) => {
            out.push_str("Data(bytes: [");
            for b in bytes {
                out.push_str(&format!("0x{b:02x},"));
            }
            out.push_str("])");
        }
        Value::Array(arr) => {
            out.push_str("[\n");
            let inner_pad = "    ".repeat(indent + 1);
            for item in arr {
                out.push_str(&inner_pad);
                swift_value(out, item, indent + 1);
                out.push_str(",\n");
            }
            out.push_str(&pad);
            out.push(']');
        }
        Value::Dictionary(dict) => {
            out.push_str("[\n");
            let inner_pad = "    ".repeat(indent + 1);
            let mut entries: Vec<(&str, &Value)> = dict.iter().collect();
            entries.sort_by(|(a, _), (b, _)| natural_cmp(a, b));
            for (key, val) in &entries {
                out.push_str(&inner_pad);
                out.push_str(&format!("{} : ", swift_string_literal(key)));
                swift_value(out, val, indent + 1);
                out.push_str(",\n");
            }
            out.push_str(&pad);
            out.push(']');
        }
    }
}

fn swift_string_literal(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    result.push('"');
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\0' => result.push_str("\\0"),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

fn value_contains_objc_invalid(value: &Value) -> bool {
    match value {
        Value::Date(_) | Value::Data(_) => true,
        Value::Array(arr) => arr.iter().any(value_contains_objc_invalid),
        Value::Dictionary(dict) => dict.iter().any(|(_, v)| value_contains_objc_invalid(v)),
        _ => false,
    }
}

fn value_to_objc(value: &Value, source_path: &str, with_import: bool) -> String {
    let filename = Path::new(source_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "plist".to_string());
    let var = var_name_from_path(source_path);

    let mut out = String::new();
    if with_import {
        out.push_str(&format!("#import \"{var}.h\"\n\n"));
    }
    out.push_str(&format!("/// Generated from {filename}\n"));
    out.push_str("__attribute__((visibility(\"hidden\")))\n");

    let root_type = match value {
        Value::Dictionary(_) => "NSDictionary",
        Value::Array(_) => "NSArray",
        _ => "id",
    };

    out.push_str(&format!("{root_type} * const {var} = "));
    objc_value(&mut out, value, 0);
    out.push_str(";\n");
    out
}

fn value_to_objc_header(value: &Value, source_path: &str, var: &str) -> String {
    let filename = Path::new(source_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "plist".to_string());

    let root_type = match value {
        Value::Dictionary(_) => "NSDictionary",
        Value::Array(_) => "NSArray",
        _ => "id",
    };

    format!(
        "#import <Foundation/Foundation.h>\n\n/// Generated from {filename}\n__attribute__((visibility(\"hidden\")))\nextern {root_type} * const {var};\n"
    )
}

fn objc_value(out: &mut String, value: &Value, indent: usize) {
    let pad = "    ".repeat(indent);
    match value {
        Value::String(s) => out.push_str(&objc_string_literal(s)),
        Value::Integer(i) => out.push_str(&format!("@{i}")),
        Value::Real(f) => {
            let mut buf = [0u8; 64];
            let fmt = b"%.6f\0";
            let len = unsafe {
                libc::snprintf(
                    buf.as_mut_ptr() as *mut libc::c_char,
                    buf.len(),
                    fmt.as_ptr() as *const libc::c_char,
                    *f,
                )
            };
            let len = (len as usize).min(buf.len() - 1);
            out.push_str(&format!("@{}", std::str::from_utf8(&buf[..len]).unwrap_or("0")));
        }
        Value::Boolean(b) => out.push_str(if *b { "@YES" } else { "@NO" }),
        Value::Date(_) | Value::Data(_) => out.push_str("nil"),
        Value::Array(arr) => {
            out.push_str("@[\n");
            let inner_pad = "    ".repeat(indent + 1);
            for item in arr {
                out.push_str(&inner_pad);
                objc_value(out, item, indent + 1);
                out.push_str(",\n");
            }
            out.push_str(&pad);
            out.push(']');
        }
        Value::Dictionary(dict) => {
            out.push_str("@{\n");
            let inner_pad = "    ".repeat(indent + 1);
            let mut entries: Vec<(&str, &Value)> = dict.iter().collect();
            entries.sort_by(|(a, _), (b, _)| natural_cmp(a, b));
            for (key, val) in &entries {
                out.push_str(&inner_pad);
                out.push_str(&format!("{} : ", objc_string_literal(key)));
                objc_value(out, val, indent + 1);
                out.push_str(",\n");
            }
            out.push_str(&pad);
            out.push('}');
        }
    }
}

fn objc_string_literal(s: &str) -> String {
    // ObjC string literal: @"..." with C-style escaping
    // Note: real newlines are kept as-is (not escaped) matching Apple's output
    let mut result = String::with_capacity(s.len() + 3);
    result.push_str("@\"");
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\0' => result.push_str("\\0"),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

fn pretty_print(value: &Value, indent: usize) {
    let pad = "  ".repeat(indent);
    match value {
        Value::String(s) => print!("\"{s}\""),
        Value::Integer(i) => print!("{i}"),
        Value::Real(f) => {
            // -p uses a shorter format
            if *f == f.floor() && f.abs() < 1e15 {
                print!("{}", *f as i64);
            } else {
                print!("{f}");
            }
        }
        Value::Boolean(b) => print!("{}", if *b { "true" } else { "false" }),
        Value::Date(abs) => {
            let unix = (*abs + CF_EPOCH_OFFSET) as i64;
            let mut tm: libc::tm = unsafe { std::mem::zeroed() };
            unsafe { libc::gmtime_r(&unix, &mut tm) };
            print!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02} +0000",
                tm.tm_year + 1900,
                tm.tm_mon + 1,
                tm.tm_mday,
                tm.tm_hour,
                tm.tm_min,
                tm.tm_sec
            );
        }
        Value::Data(bytes) => {
            print!("{{length = {}, bytes = 0x", bytes.len());
            for b in bytes {
                print!("{b:x}");
            }
            print!("}}");
        }
        Value::Array(arr) => {
            println!("[");
            for (i, item) in arr.iter().enumerate() {
                print!("{pad}  {i} => ");
                pretty_print(item, indent + 1);
                println!();
            }
            print!("{pad}]");
        }
        Value::Dictionary(dict) => {
            println!("{{");
            // Sort keys for -p output
            let mut entries: Vec<(&str, &Value)> = dict.iter().collect();
            entries.sort_by(|(a, _), (b, _)| natural_cmp(a, b));
            for (key, val) in &entries {
                print!("{pad}  \"{key}\" => ");
                pretty_print(val, indent + 1);
                println!();
            }
            print!("{pad}}}");
        }
    }
    if indent == 0 {
        println!();
    }
}
