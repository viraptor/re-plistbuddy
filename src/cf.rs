use std::ffi::CStr;
use std::path::Path;
use std::ptr;

use crate::value::{Dictionary, Value};

// Core Foundation type aliases
type CFTypeRef = *const std::ffi::c_void;
type CFAllocatorRef = *const std::ffi::c_void;
type CFStringRef = *const std::ffi::c_void;
type CFDataRef = *const std::ffi::c_void;
type CFDictionaryRef = *const std::ffi::c_void;
type CFMutableDictionaryRef = *mut std::ffi::c_void;
type CFArrayRef = *const std::ffi::c_void;
type CFMutableArrayRef = *mut std::ffi::c_void;
type CFNumberRef = *const std::ffi::c_void;
type CFBooleanRef = *const std::ffi::c_void;
type CFDateRef = *const std::ffi::c_void;
type CFURLRef = *const std::ffi::c_void;
type CFErrorRef = *mut std::ffi::c_void;
type CFWriteStreamRef = *mut std::ffi::c_void;
type CFReadStreamRef = *mut std::ffi::c_void;
type CFPropertyListRef = CFTypeRef;
type CFIndex = isize;
type CFTypeID = usize;
type CFAbsoluteTime = f64;
type CFStringEncoding = u32;
type CFNumberType = i32;
type CFPropertyListFormat = i32;
type CFPropertyListMutabilityOptions = u32;
type Boolean = u8;

const K_CF_STRING_ENCODING_UTF8: CFStringEncoding = 0x08000100;
const K_CF_NUMBER_SINT64_TYPE: CFNumberType = 4;
const K_CF_NUMBER_FLOAT64_TYPE: CFNumberType = 6;
const K_CF_PROPERTY_LIST_XML_FORMAT_V1_0: CFPropertyListFormat = 100;
const K_CF_PROPERTY_LIST_BINARY_FORMAT_V1_0: CFPropertyListFormat = 200;
const K_CF_PROPERTY_LIST_MUTABLE_CONTAINERS_AND_LEAVES: CFPropertyListMutabilityOptions = 2;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    // Memory management
    fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    fn CFRelease(cf: CFTypeRef);
    fn CFGetTypeID(cf: CFTypeRef) -> CFTypeID;

    // Type IDs
    fn CFStringGetTypeID() -> CFTypeID;
    fn CFNumberGetTypeID() -> CFTypeID;
    fn CFBooleanGetTypeID() -> CFTypeID;
    fn CFDateGetTypeID() -> CFTypeID;
    fn CFDataGetTypeID() -> CFTypeID;
    fn CFDictionaryGetTypeID() -> CFTypeID;
    fn CFArrayGetTypeID() -> CFTypeID;

    // String
    fn CFStringCreateWithCString(
        alloc: CFAllocatorRef,
        c_str: *const i8,
        encoding: CFStringEncoding,
    ) -> CFStringRef;
    fn CFStringGetLength(s: CFStringRef) -> CFIndex;
    fn CFStringGetCString(
        s: CFStringRef,
        buffer: *mut i8,
        buffer_size: CFIndex,
        encoding: CFStringEncoding,
    ) -> Boolean;
    fn CFStringGetMaximumSizeForEncoding(
        length: CFIndex,
        encoding: CFStringEncoding,
    ) -> CFIndex;

    // Number
    fn CFNumberCreate(
        alloc: CFAllocatorRef,
        the_type: CFNumberType,
        value_ptr: *const std::ffi::c_void,
    ) -> CFNumberRef;
    fn CFNumberGetValue(
        number: CFNumberRef,
        the_type: CFNumberType,
        value_ptr: *mut std::ffi::c_void,
    ) -> Boolean;
    fn CFNumberIsFloatType(number: CFNumberRef) -> Boolean;

    // Boolean
    static kCFBooleanTrue: CFBooleanRef;
    static kCFBooleanFalse: CFBooleanRef;
    fn CFBooleanGetValue(boolean: CFBooleanRef) -> Boolean;

    // Date
    fn CFDateCreate(alloc: CFAllocatorRef, at: CFAbsoluteTime) -> CFDateRef;
    fn CFDateGetAbsoluteTime(date: CFDateRef) -> CFAbsoluteTime;

    // Data
    fn CFDataCreate(
        alloc: CFAllocatorRef,
        bytes: *const u8,
        length: CFIndex,
    ) -> CFDataRef;
    fn CFDataGetBytePtr(data: CFDataRef) -> *const u8;
    fn CFDataGetLength(data: CFDataRef) -> CFIndex;

    // Dictionary
    fn CFDictionaryCreateMutable(
        alloc: CFAllocatorRef,
        capacity: CFIndex,
        key_callbacks: *const std::ffi::c_void,
        value_callbacks: *const std::ffi::c_void,
    ) -> CFMutableDictionaryRef;
    fn CFDictionaryGetCount(dict: CFDictionaryRef) -> CFIndex;
    fn CFDictionaryGetKeysAndValues(
        dict: CFDictionaryRef,
        keys: *mut CFTypeRef,
        values: *mut CFTypeRef,
    );
    fn CFDictionarySetValue(
        dict: CFMutableDictionaryRef,
        key: CFTypeRef,
        value: CFTypeRef,
    );

    // Array
    fn CFArrayCreateMutable(
        alloc: CFAllocatorRef,
        capacity: CFIndex,
        callbacks: *const std::ffi::c_void,
    ) -> CFMutableArrayRef;
    fn CFArrayGetCount(array: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: CFIndex) -> CFTypeRef;
    fn CFArrayAppendValue(array: CFMutableArrayRef, value: CFTypeRef);

    // Error
    fn CFErrorCopyDescription(err: CFErrorRef) -> CFStringRef;
    fn CFErrorCopyUserInfo(err: CFErrorRef) -> CFDictionaryRef;
    fn CFDictionaryGetValue(dict: CFDictionaryRef, key: CFTypeRef) -> CFTypeRef;

    // Dictionary/Array callback constants
    static kCFTypeDictionaryKeyCallBacks: std::ffi::c_void;
    static kCFTypeDictionaryValueCallBacks: std::ffi::c_void;
    static kCFTypeArrayCallBacks: std::ffi::c_void;

    // PropertyList
    fn CFPropertyListCreateWithStream(
        alloc: CFAllocatorRef,
        stream: CFReadStreamRef,
        stream_length: CFIndex,
        options: CFPropertyListMutabilityOptions,
        format: *mut CFPropertyListFormat,
        error: *mut CFErrorRef,
    ) -> CFPropertyListRef;
    fn CFPropertyListWrite(
        property_list: CFPropertyListRef,
        stream: CFWriteStreamRef,
        format: CFPropertyListFormat,
        options: u32,
        error: *mut CFErrorRef,
    ) -> CFIndex;

    // URL
    fn CFURLCreateWithFileSystemPath(
        alloc: CFAllocatorRef,
        file_path: CFStringRef,
        path_style: i32,
        is_directory: Boolean,
    ) -> CFURLRef;

    // Stream
    fn CFReadStreamCreateWithFile(alloc: CFAllocatorRef, file_url: CFURLRef) -> CFReadStreamRef;
    fn CFWriteStreamCreateWithFile(alloc: CFAllocatorRef, file_url: CFURLRef) -> CFWriteStreamRef;
    fn CFReadStreamOpen(stream: CFReadStreamRef) -> Boolean;
    fn CFWriteStreamOpen(stream: CFWriteStreamRef) -> Boolean;
    fn CFReadStreamClose(stream: CFReadStreamRef);
    fn CFWriteStreamClose(stream: CFWriteStreamRef);

    // In-memory serialization
    fn CFPropertyListCreateData(
        alloc: CFAllocatorRef,
        property_list: CFPropertyListRef,
        format: CFPropertyListFormat,
        options: u32,
        error: *mut CFErrorRef,
    ) -> CFDataRef;
}

// POSIX path style
const K_CF_URL_POSIX_PATH_STYLE: i32 = 0;

// Safe wrapper around CFTypeRef that auto-releases
struct CfRef {
    ptr: CFTypeRef,
}

impl CfRef {
    fn new(ptr: CFTypeRef) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(CfRef { ptr })
        }
    }

    fn as_ptr(&self) -> CFTypeRef {
        self.ptr
    }
}

impl Drop for CfRef {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { CFRelease(self.ptr) };
        }
    }
}

fn cf_error_description(error: CFErrorRef) -> Option<String> {
    if error.is_null() {
        return None;
    }
    unsafe {
        // Try NSDebugDescription from userInfo (most specific, matches PlistBuddy)
        let user_info = CFErrorCopyUserInfo(error);
        if !user_info.is_null() {
            let debug_key = cf_string_from_str("NSDebugDescription");
            if let Some(ref key) = debug_key {
                let debug_val = CFDictionaryGetValue(user_info, key.as_ptr());
                if !debug_val.is_null() && CFGetTypeID(debug_val) == CFStringGetTypeID() {
                    let result = cf_string_to_string(debug_val);
                    CFRelease(user_info as CFTypeRef);
                    if result.is_some() {
                        return result;
                    }
                }
            }
            CFRelease(user_info as CFTypeRef);
        }
        // Fall back to description
        let desc = CFErrorCopyDescription(error);
        if desc.is_null() {
            return None;
        }
        let result = cf_string_to_string(desc);
        CFRelease(desc);
        result
    }
}

fn cf_string_from_str(s: &str) -> Option<CfRef> {
    let c_str = std::ffi::CString::new(s).ok()?;
    let cf = unsafe {
        CFStringCreateWithCString(ptr::null(), c_str.as_ptr(), K_CF_STRING_ENCODING_UTF8)
    };
    CfRef::new(cf)
}

fn cf_string_to_string(cf_str: CFStringRef) -> Option<String> {
    if cf_str.is_null() {
        return None;
    }
    unsafe {
        let len = CFStringGetLength(cf_str);
        let max_size = CFStringGetMaximumSizeForEncoding(len, K_CF_STRING_ENCODING_UTF8) + 1;
        let mut buf = vec![0i8; max_size as usize];
        if CFStringGetCString(cf_str, buf.as_mut_ptr(), max_size, K_CF_STRING_ENCODING_UTF8) != 0 {
            let cstr = CStr::from_ptr(buf.as_ptr());
            Some(cstr.to_string_lossy().into_owned())
        } else {
            None
        }
    }
}

/// Convert a CFTypeRef to our Value enum
fn cf_to_value(cf: CFTypeRef) -> anyhow::Result<Value> {
    if cf.is_null() {
        anyhow::bail!("null CFTypeRef");
    }
    unsafe {
        let type_id = CFGetTypeID(cf);

        if type_id == CFStringGetTypeID() {
            let s = cf_string_to_string(cf).unwrap_or_default();
            Ok(Value::String(s))
        } else if type_id == CFNumberGetTypeID() {
            if CFNumberIsFloatType(cf) != 0 {
                let mut val: f64 = 0.0;
                CFNumberGetValue(
                    cf,
                    K_CF_NUMBER_FLOAT64_TYPE,
                    &mut val as *mut f64 as *mut std::ffi::c_void,
                );
                Ok(Value::Real(val))
            } else {
                let mut val: i64 = 0;
                CFNumberGetValue(
                    cf,
                    K_CF_NUMBER_SINT64_TYPE,
                    &mut val as *mut i64 as *mut std::ffi::c_void,
                );
                Ok(Value::Integer(val))
            }
        } else if type_id == CFBooleanGetTypeID() {
            let val = CFBooleanGetValue(cf) != 0;
            Ok(Value::Boolean(val))
        } else if type_id == CFDateGetTypeID() {
            let abs_time = CFDateGetAbsoluteTime(cf);
            Ok(Value::Date(abs_time))
        } else if type_id == CFDataGetTypeID() {
            let ptr = CFDataGetBytePtr(cf);
            let len = CFDataGetLength(cf) as usize;
            let bytes = if ptr.is_null() || len == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(ptr, len).to_vec()
            };
            Ok(Value::Data(bytes))
        } else if type_id == CFDictionaryGetTypeID() {
            let count = CFDictionaryGetCount(cf) as usize;
            let mut keys = vec![ptr::null() as CFTypeRef; count];
            let mut values = vec![ptr::null() as CFTypeRef; count];
            CFDictionaryGetKeysAndValues(cf, keys.as_mut_ptr(), values.as_mut_ptr());

            let mut dict = Dictionary::new();
            for i in 0..count {
                let key_str = cf_string_to_string(keys[i]).unwrap_or_default();
                let val = cf_to_value(values[i])?;
                dict.insert(key_str, val);
            }
            Ok(Value::Dictionary(dict))
        } else if type_id == CFArrayGetTypeID() {
            let count = CFArrayGetCount(cf) as usize;
            let mut arr = Vec::with_capacity(count);
            for i in 0..count {
                let item = CFArrayGetValueAtIndex(cf, i as CFIndex);
                arr.push(cf_to_value(item)?);
            }
            Ok(Value::Array(arr))
        } else {
            anyhow::bail!("unknown CFTypeRef type ID: {type_id}");
        }
    }
}

/// Convert our Value to a CFTypeRef (caller must CFRelease)
fn value_to_cf(value: &Value) -> anyhow::Result<CfRef> {
    unsafe {
        match value {
            Value::String(s) => {
                cf_string_from_str(s)
                    .ok_or_else(|| anyhow::anyhow!("failed to create CFString"))
            }
            Value::Integer(v) => {
                let cf = CFNumberCreate(
                    ptr::null(),
                    K_CF_NUMBER_SINT64_TYPE,
                    v as *const i64 as *const std::ffi::c_void,
                );
                CfRef::new(cf).ok_or_else(|| anyhow::anyhow!("failed to create CFNumber"))
            }
            Value::Real(v) => {
                let cf = CFNumberCreate(
                    ptr::null(),
                    K_CF_NUMBER_FLOAT64_TYPE,
                    v as *const f64 as *const std::ffi::c_void,
                );
                CfRef::new(cf).ok_or_else(|| anyhow::anyhow!("failed to create CFNumber"))
            }
            Value::Boolean(v) => {
                let cf = if *v { kCFBooleanTrue } else { kCFBooleanFalse };
                CFRetain(cf);
                CfRef::new(cf).ok_or_else(|| anyhow::anyhow!("failed to create CFBoolean"))
            }
            Value::Date(abs_time) => {
                let cf = CFDateCreate(ptr::null(), *abs_time);
                CfRef::new(cf).ok_or_else(|| anyhow::anyhow!("failed to create CFDate"))
            }
            Value::Data(bytes) => {
                let cf = CFDataCreate(
                    ptr::null(),
                    bytes.as_ptr(),
                    bytes.len() as CFIndex,
                );
                CfRef::new(cf).ok_or_else(|| anyhow::anyhow!("failed to create CFData"))
            }
            Value::Dictionary(dict) => {
                let cf_dict = CFDictionaryCreateMutable(
                    ptr::null(),
                    0,
                    &kCFTypeDictionaryKeyCallBacks,
                    &kCFTypeDictionaryValueCallBacks,
                );
                if cf_dict.is_null() {
                    anyhow::bail!("failed to create CFMutableDictionary");
                }
                let dict_ref = CfRef::new(cf_dict as CFTypeRef).unwrap();
                for (key, val) in dict.iter() {
                    let cf_key = cf_string_from_str(key)
                        .ok_or_else(|| anyhow::anyhow!("failed to create key CFString"))?;
                    let cf_val = value_to_cf(val)?;
                    CFDictionarySetValue(
                        dict_ref.as_ptr() as CFMutableDictionaryRef,
                        cf_key.as_ptr(),
                        cf_val.as_ptr(),
                    );
                }
                Ok(dict_ref)
            }
            Value::Array(arr) => {
                let cf_arr = CFArrayCreateMutable(
                    ptr::null(),
                    0,
                    &kCFTypeArrayCallBacks,
                );
                if cf_arr.is_null() {
                    anyhow::bail!("failed to create CFMutableArray");
                }
                let arr_ref = CfRef::new(cf_arr as CFTypeRef).unwrap();
                for item in arr {
                    let cf_item = value_to_cf(item)?;
                    CFArrayAppendValue(
                        arr_ref.as_ptr() as CFMutableArrayRef,
                        cf_item.as_ptr(),
                    );
                }
                Ok(arr_ref)
            }
        }
    }
}

pub fn read_plist(path: &Path) -> anyhow::Result<Value> {
    let path_str = path.to_str().ok_or_else(|| anyhow::anyhow!("invalid path"))?;
    let cf_path = cf_string_from_str(path_str)
        .ok_or_else(|| anyhow::anyhow!("failed to create path CFString"))?;

    unsafe {
        let url = CFURLCreateWithFileSystemPath(
            ptr::null(),
            cf_path.as_ptr(),
            K_CF_URL_POSIX_PATH_STYLE,
            0,
        );
        let url_ref = CfRef::new(url)
            .ok_or_else(|| anyhow::anyhow!("failed to create CFURL"))?;

        let stream = CFReadStreamCreateWithFile(ptr::null(), url_ref.as_ptr());
        let stream_ref = CfRef::new(stream as CFTypeRef)
            .ok_or_else(|| anyhow::anyhow!("failed to create read stream"))?;

        if CFReadStreamOpen(stream_ref.as_ptr() as CFReadStreamRef) == 0 {
            anyhow::bail!("failed to open read stream");
        }

        let mut format: CFPropertyListFormat = 0;
        let mut error: CFErrorRef = ptr::null_mut();
        let plist = CFPropertyListCreateWithStream(
            ptr::null(),
            stream_ref.as_ptr() as CFReadStreamRef,
            0,
            K_CF_PROPERTY_LIST_MUTABLE_CONTAINERS_AND_LEAVES,
            &mut format,
            &mut error,
        );

        CFReadStreamClose(stream_ref.as_ptr() as CFReadStreamRef);

        if !error.is_null() {
            let desc = cf_error_description(error);
            CFRelease(error as CFTypeRef);
            if plist.is_null() {
                anyhow::bail!("{}", desc.unwrap_or_else(|| "failed to parse plist".to_string()));
            }
        }

        let plist_ref = CfRef::new(plist)
            .ok_or_else(|| anyhow::anyhow!("failed to parse plist"))?;

        cf_to_value(plist_ref.as_ptr())
    }
}

fn write_plist(value: &Value, path: &Path, format: CFPropertyListFormat) -> anyhow::Result<()> {
    let cf_value = value_to_cf(value)?;

    let path_str = path.to_str().ok_or_else(|| anyhow::anyhow!("invalid path"))?;
    let cf_path = cf_string_from_str(path_str)
        .ok_or_else(|| anyhow::anyhow!("failed to create path CFString"))?;

    unsafe {
        let url = CFURLCreateWithFileSystemPath(
            ptr::null(),
            cf_path.as_ptr(),
            K_CF_URL_POSIX_PATH_STYLE,
            0,
        );
        let url_ref = CfRef::new(url)
            .ok_or_else(|| anyhow::anyhow!("failed to create CFURL"))?;

        let stream = CFWriteStreamCreateWithFile(ptr::null(), url_ref.as_ptr());
        let stream_ref = CfRef::new(stream as CFTypeRef)
            .ok_or_else(|| anyhow::anyhow!("failed to create write stream"))?;

        if CFWriteStreamOpen(stream_ref.as_ptr() as CFWriteStreamRef) == 0 {
            anyhow::bail!("failed to open write stream");
        }

        let mut error: CFErrorRef = ptr::null_mut();
        let written = CFPropertyListWrite(
            cf_value.as_ptr(),
            stream_ref.as_ptr() as CFWriteStreamRef,
            format,
            0,
            &mut error,
        );

        CFWriteStreamClose(stream_ref.as_ptr() as CFWriteStreamRef);

        if !error.is_null() {
            CFRelease(error as CFTypeRef);
        }

        if written == 0 {
            anyhow::bail!("failed to write plist");
        }

        Ok(())
    }
}

pub fn write_plist_xml(value: &Value, path: &Path) -> anyhow::Result<()> {
    write_plist(value, path, K_CF_PROPERTY_LIST_XML_FORMAT_V1_0)
}

pub fn write_plist_binary(value: &Value, path: &Path) -> anyhow::Result<()> {
    write_plist(value, path, K_CF_PROPERTY_LIST_BINARY_FORMAT_V1_0)
}

pub fn value_to_xml_bytes(value: &Value) -> anyhow::Result<Vec<u8>> {
    let cf_value = value_to_cf(value)?;

    unsafe {
        let mut error: CFErrorRef = ptr::null_mut();
        let data = CFPropertyListCreateData(
            ptr::null(),
            cf_value.as_ptr(),
            K_CF_PROPERTY_LIST_XML_FORMAT_V1_0,
            0,
            &mut error,
        );

        if !error.is_null() {
            CFRelease(error as CFTypeRef);
        }

        let data_ref = CfRef::new(data)
            .ok_or_else(|| anyhow::anyhow!("failed to create XML data"))?;

        let ptr = CFDataGetBytePtr(data_ref.as_ptr());
        let len = CFDataGetLength(data_ref.as_ptr()) as usize;
        if ptr.is_null() {
            anyhow::bail!("null data pointer");
        }
        Ok(std::slice::from_raw_parts(ptr, len).to_vec())
    }
}
