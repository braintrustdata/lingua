use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

// Import our types and conversion traits
use crate::providers::anthropic::generated as anthropic;
use crate::providers::openai::generated as openai;
use crate::universal::{convert::TryFromLLM, Message};

// ============================================================================
// C-compatible string helpers
// ============================================================================

/// Convert a C string to a Rust String
/// Returns None if the pointer is null
unsafe fn c_str_to_string(c_str: *const c_char) -> Option<String> {
    if c_str.is_null() {
        return None;
    }
    CStr::from_ptr(c_str).to_str().ok().map(|s| s.to_string())
}

/// Convert a Rust String to a C string
/// Returns null pointer if the string cannot be converted
fn string_to_c_str(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c_string) => c_string.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a C string that was allocated by Rust
#[no_mangle]
pub extern "C" fn lingua_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

// ============================================================================
// Generic conversion functions
// ============================================================================

/// Convert JSON string from provider format to Lingua format
fn convert_to_lingua<T, U>(json: &str) -> Result<String, String>
where
    T: for<'de> Deserialize<'de>,
    U: TryFromLLM<T> + Serialize,
    <U as TryFromLLM<T>>::Error: std::fmt::Debug,
{
    let provider_msg: T =
        serde_json::from_str(json).map_err(|e| format!("Failed to parse input JSON: {}", e))?;

    let lingua_msg = U::try_from(provider_msg).map_err(|e| format!("Conversion error: {:?}", e))?;

    serde_json::to_string(&lingua_msg).map_err(|e| format!("Failed to serialize result: {}", e))
}

/// Convert JSON string from Lingua format to provider format
fn convert_from_lingua<T, U>(json: &str) -> Result<String, String>
where
    T: for<'de> Deserialize<'de>,
    U: TryFromLLM<T> + Serialize,
    <U as TryFromLLM<T>>::Error: std::fmt::Debug,
{
    let lingua_msg: T =
        serde_json::from_str(json).map_err(|e| format!("Failed to parse input JSON: {}", e))?;

    let provider_msg = U::try_from(lingua_msg).map_err(|e| format!("Conversion error: {:?}", e))?;

    serde_json::to_string(&provider_msg).map_err(|e| format!("Failed to serialize result: {}", e))
}

// ============================================================================
// Chat Completions API conversions
// ============================================================================

/// Convert array of Chat Completions messages to Lingua Messages
///
/// # Arguments
/// * `json` - JSON string containing array of ChatCompletionRequestMessage
/// * `error_out` - Output parameter for error message (null if successful)
///
/// # Returns
/// JSON string containing array of Lingua Messages, or null on error
#[no_mangle]
pub extern "C" fn lingua_chat_completions_to_lingua(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    match convert_to_lingua::<Vec<openai::ChatCompletionRequestMessage>, Vec<Message>>(&json_str) {
        Ok(result) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = ptr::null_mut();
                }
            }
            string_to_c_str(result)
        }
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(e);
                }
            }
            ptr::null_mut()
        }
    }
}

/// Convert array of Lingua Messages to Chat Completions messages
#[no_mangle]
pub extern "C" fn lingua_to_chat_completions(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    match convert_from_lingua::<Vec<Message>, Vec<openai::ChatCompletionRequestMessage>>(&json_str)
    {
        Ok(result) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = ptr::null_mut();
                }
            }
            string_to_c_str(result)
        }
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(e);
                }
            }
            ptr::null_mut()
        }
    }
}

// ============================================================================
// Responses API conversions
// ============================================================================

/// Convert array of Responses API messages to Lingua Messages
#[no_mangle]
pub extern "C" fn lingua_responses_to_lingua(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    match convert_to_lingua::<Vec<openai::InputItem>, Vec<Message>>(&json_str) {
        Ok(result) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = ptr::null_mut();
                }
            }
            string_to_c_str(result)
        }
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(e);
                }
            }
            ptr::null_mut()
        }
    }
}

/// Convert array of Lingua Messages to Responses API messages
#[no_mangle]
pub extern "C" fn lingua_to_responses(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    match convert_from_lingua::<Vec<Message>, Vec<openai::InputItem>>(&json_str) {
        Ok(result) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = ptr::null_mut();
                }
            }
            string_to_c_str(result)
        }
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(e);
                }
            }
            ptr::null_mut()
        }
    }
}

// ============================================================================
// Anthropic conversions
// ============================================================================

/// Convert array of Anthropic messages to Lingua Messages
#[no_mangle]
pub extern "C" fn lingua_anthropic_to_lingua(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    match convert_to_lingua::<Vec<anthropic::InputMessage>, Vec<Message>>(&json_str) {
        Ok(result) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = ptr::null_mut();
                }
            }
            string_to_c_str(result)
        }
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(e);
                }
            }
            ptr::null_mut()
        }
    }
}

/// Convert array of Lingua Messages to Anthropic messages
#[no_mangle]
pub extern "C" fn lingua_to_anthropic(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    match convert_from_lingua::<Vec<Message>, Vec<anthropic::InputMessage>>(&json_str) {
        Ok(result) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = ptr::null_mut();
                }
            }
            string_to_c_str(result)
        }
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(e);
                }
            }
            ptr::null_mut()
        }
    }
}

// ============================================================================
// Processing functions
// ============================================================================

/// Deduplicate messages based on role and content
#[no_mangle]
pub extern "C" fn lingua_deduplicate_messages(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    use crate::processing::dedup::deduplicate_messages as dedup;

    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    let messages: Vec<Message> = match serde_json::from_str(&json_str) {
        Ok(m) => m,
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(format!("Failed to parse messages: {}", e));
                }
            }
            return ptr::null_mut();
        }
    };

    let deduplicated = dedup(messages);

    match serde_json::to_string(&deduplicated) {
        Ok(result) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = ptr::null_mut();
                }
            }
            string_to_c_str(result)
        }
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(format!("Failed to serialize result: {}", e));
                }
            }
            ptr::null_mut()
        }
    }
}

// ============================================================================
// Validation functions
// ============================================================================

/// Validate a JSON string as a Chat Completions request
#[no_mangle]
#[cfg(feature = "openai")]
pub extern "C" fn lingua_validate_chat_completions_request(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    use crate::validation::openai::validate_chat_completions_request as validate;

    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    match validate(&json_str) {
        Ok(req) => match serde_json::to_string(&req) {
            Ok(result) => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = ptr::null_mut();
                    }
                }
                string_to_c_str(result)
            }
            Err(e) => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str(format!("Failed to serialize result: {}", e));
                    }
                }
                ptr::null_mut()
            }
        },
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(e.to_string());
                }
            }
            ptr::null_mut()
        }
    }
}

/// Validate a JSON string as a Chat Completions response
#[no_mangle]
#[cfg(feature = "openai")]
pub extern "C" fn lingua_validate_chat_completions_response(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    use crate::validation::openai::validate_chat_completions_response as validate;

    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    match validate(&json_str) {
        Ok(res) => match serde_json::to_string(&res) {
            Ok(result) => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = ptr::null_mut();
                    }
                }
                string_to_c_str(result)
            }
            Err(e) => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str(format!("Failed to serialize result: {}", e));
                    }
                }
                ptr::null_mut()
            }
        },
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(e.to_string());
                }
            }
            ptr::null_mut()
        }
    }
}

/// Validate a JSON string as a Responses API request
#[no_mangle]
#[cfg(feature = "openai")]
pub extern "C" fn lingua_validate_responses_request(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    use crate::validation::openai::validate_responses_request as validate;

    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    match validate(&json_str) {
        Ok(req) => match serde_json::to_string(&req) {
            Ok(result) => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = ptr::null_mut();
                    }
                }
                string_to_c_str(result)
            }
            Err(e) => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str(format!("Failed to serialize result: {}", e));
                    }
                }
                ptr::null_mut()
            }
        },
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(e.to_string());
                }
            }
            ptr::null_mut()
        }
    }
}

/// Validate a JSON string as a Responses API response
#[no_mangle]
#[cfg(feature = "openai")]
pub extern "C" fn lingua_validate_responses_response(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    use crate::validation::openai::validate_responses_response as validate;

    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    match validate(&json_str) {
        Ok(res) => match serde_json::to_string(&res) {
            Ok(result) => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = ptr::null_mut();
                    }
                }
                string_to_c_str(result)
            }
            Err(e) => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str(format!("Failed to serialize result: {}", e));
                    }
                }
                ptr::null_mut()
            }
        },
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(e.to_string());
                }
            }
            ptr::null_mut()
        }
    }
}

/// Validate a JSON string as an Anthropic request
#[no_mangle]
#[cfg(feature = "anthropic")]
pub extern "C" fn lingua_validate_anthropic_request(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    use crate::validation::anthropic::validate_anthropic_request as validate;

    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    match validate(&json_str) {
        Ok(req) => match serde_json::to_string(&req) {
            Ok(result) => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = ptr::null_mut();
                    }
                }
                string_to_c_str(result)
            }
            Err(e) => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str(format!("Failed to serialize result: {}", e));
                    }
                }
                ptr::null_mut()
            }
        },
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(e.to_string());
                }
            }
            ptr::null_mut()
        }
    }
}

/// Validate a JSON string as an Anthropic response
#[no_mangle]
#[cfg(feature = "anthropic")]
pub extern "C" fn lingua_validate_anthropic_response(
    json: *const c_char,
    error_out: *mut *mut c_char,
) -> *mut c_char {
    use crate::validation::anthropic::validate_anthropic_response as validate;

    let json_str = unsafe {
        match c_str_to_string(json) {
            Some(s) => s,
            None => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str("Input JSON is null".to_string());
                    }
                }
                return ptr::null_mut();
            }
        }
    };

    match validate(&json_str) {
        Ok(res) => match serde_json::to_string(&res) {
            Ok(result) => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = ptr::null_mut();
                    }
                }
                string_to_c_str(result)
            }
            Err(e) => {
                if !error_out.is_null() {
                    unsafe {
                        *error_out = string_to_c_str(format!("Failed to serialize result: {}", e));
                    }
                }
                ptr::null_mut()
            }
        },
        Err(e) => {
            if !error_out.is_null() {
                unsafe {
                    *error_out = string_to_c_str(e.to_string());
                }
            }
            ptr::null_mut()
        }
    }
}
