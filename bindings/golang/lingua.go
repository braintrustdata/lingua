// Package lingua provides Go bindings for the Lingua universal message format library.
//
// Lingua is a universal message format that compiles to provider-specific formats
// with zero runtime overhead. It enables seamless interoperability between different
// LLM providers (OpenAI, Anthropic, etc.) through compile-time translation.
//
// This package wraps the Rust implementation of Lingua using CGo and provides
// idiomatic Go functions for message conversion, validation, and processing.
package lingua

/*
#cgo LDFLAGS: -L../../target/release -llingua -ldl -lm -lpthread
#include <stdlib.h>

// Forward declarations of Rust FFI functions
extern char* lingua_chat_completions_to_lingua(const char* json, char** error_out);
extern char* lingua_to_chat_completions(const char* json, char** error_out);
extern char* lingua_responses_to_lingua(const char* json, char** error_out);
extern char* lingua_to_responses(const char* json, char** error_out);
extern char* lingua_anthropic_to_lingua(const char* json, char** error_out);
extern char* lingua_to_anthropic(const char* json, char** error_out);
extern char* lingua_deduplicate_messages(const char* json, char** error_out);
extern char* lingua_import_messages_from_spans(const char* json, char** error_out);
extern char* lingua_validate_chat_completions_request(const char* json, char** error_out);
extern char* lingua_validate_chat_completions_response(const char* json, char** error_out);
extern char* lingua_validate_responses_request(const char* json, char** error_out);
extern char* lingua_validate_responses_response(const char* json, char** error_out);
extern char* lingua_validate_anthropic_request(const char* json, char** error_out);
extern char* lingua_validate_anthropic_response(const char* json, char** error_out);
extern void lingua_free_string(char* s);
*/
import "C"
import (
	"encoding/json"
	"errors"
	"unsafe"
)

// ConversionError represents an error during format conversion.
type ConversionError struct {
	Message  string
	Provider string
}

func (e *ConversionError) Error() string {
	if e.Provider != "" {
		return e.Provider + ": " + e.Message
	}
	return e.Message
}

// rustFunctionID identifies which Rust FFI function to call.
type rustFunctionID int

const (
	fnChatCompletionsToLingua rustFunctionID = iota
	fnLinguaToChatCompletions
	fnResponsesToLingua
	fnLinguaToResponses
	fnAnthropicToLingua
	fnLinguaToAnthropic
	fnDeduplicateMessages
	fnImportMessagesFromSpans
	fnValidateChatCompletionsRequest
	fnValidateChatCompletionsResponse
	fnValidateResponsesRequest
	fnValidateResponsesResponse
	fnValidateAnthropicRequest
	fnValidateAnthropicResponse
)

// callRustFunction is a helper to call Rust FFI functions and handle errors.
func callRustFunction(fnID rustFunctionID, input string) (string, error) {
	cInput := C.CString(input)
	defer C.free(unsafe.Pointer(cInput))

	var cError *C.char
	var cResult *C.char

	// Call the appropriate Rust FFI function based on ID
	//nolint:gocritic // CGo FFI dispatch requires enumerating each function call explicitly
	switch fnID {
	case fnChatCompletionsToLingua:
		cResult = C.lingua_chat_completions_to_lingua(cInput, &cError)
	case fnLinguaToChatCompletions:
		cResult = C.lingua_to_chat_completions(cInput, &cError)
	case fnResponsesToLingua:
		cResult = C.lingua_responses_to_lingua(cInput, &cError)
	case fnLinguaToResponses:
		cResult = C.lingua_to_responses(cInput, &cError)
	case fnAnthropicToLingua:
		cResult = C.lingua_anthropic_to_lingua(cInput, &cError)
	case fnLinguaToAnthropic:
		cResult = C.lingua_to_anthropic(cInput, &cError)
	case fnDeduplicateMessages:
		cResult = C.lingua_deduplicate_messages(cInput, &cError)
	case fnImportMessagesFromSpans:
		cResult = C.lingua_import_messages_from_spans(cInput, &cError)
	case fnValidateChatCompletionsRequest:
		cResult = C.lingua_validate_chat_completions_request(cInput, &cError)
	case fnValidateChatCompletionsResponse:
		cResult = C.lingua_validate_chat_completions_response(cInput, &cError)
	case fnValidateResponsesRequest:
		cResult = C.lingua_validate_responses_request(cInput, &cError)
	case fnValidateResponsesResponse:
		cResult = C.lingua_validate_responses_response(cInput, &cError)
	case fnValidateAnthropicRequest:
		cResult = C.lingua_validate_anthropic_request(cInput, &cError)
	case fnValidateAnthropicResponse:
		cResult = C.lingua_validate_anthropic_response(cInput, &cError)
	default:
		return "", errors.New("unknown function")
	}

	if cError != nil {
		errMsg := C.GoString(cError)
		C.lingua_free_string(cError)
		return "", errors.New(errMsg)
	}

	if cResult == nil {
		return "", errors.New("conversion failed with no error message")
	}

	result := C.GoString(cResult)
	C.lingua_free_string(cResult)
	return result, nil
}

// ============================================================================
// Chat Completions API Conversions
// ============================================================================

// ChatCompletionsMessagesToLingua converts Chat Completions messages to Lingua format.
func ChatCompletionsMessagesToLingua(messages any) ([]map[string]any, error) {
	jsonBytes, err := json.Marshal(messages)
	if err != nil {
		return nil, &ConversionError{
			Message:  "failed to marshal input: " + err.Error(),
			Provider: "Chat Completions",
		}
	}

	resultJSON, err := callRustFunction(fnChatCompletionsToLingua, string(jsonBytes))
	if err != nil {
		return nil, &ConversionError{
			Message:  err.Error(),
			Provider: "Chat Completions",
		}
	}

	var result []map[string]any
	err = json.Unmarshal([]byte(resultJSON), &result)
	if err != nil {
		return nil, &ConversionError{
			Message:  "failed to unmarshal result: " + err.Error(),
			Provider: "Chat Completions",
		}
	}

	return result, nil
}

// LinguaToChatCompletionsMessages converts Lingua messages to Chat Completions format.
//
//nolint:revive // Preserve exported name for backward compatibility
func LinguaToChatCompletionsMessages(messages any) ([]map[string]any, error) {
	jsonBytes, err := json.Marshal(messages)
	if err != nil {
		return nil, &ConversionError{
			Message:  "failed to marshal input: " + err.Error(),
			Provider: "Chat Completions",
		}
	}

	resultJSON, err := callRustFunction(fnLinguaToChatCompletions, string(jsonBytes))
	if err != nil {
		return nil, &ConversionError{
			Message:  err.Error(),
			Provider: "Chat Completions",
		}
	}

	var result []map[string]any
	err = json.Unmarshal([]byte(resultJSON), &result)
	if err != nil {
		return nil, &ConversionError{
			Message:  "failed to unmarshal result: " + err.Error(),
			Provider: "Chat Completions",
		}
	}

	return result, nil
}

// ============================================================================
// Responses API Conversions
// ============================================================================

// ResponsesMessagesToLingua converts Responses API messages to Lingua format.
func ResponsesMessagesToLingua(messages any) ([]map[string]any, error) {
	jsonBytes, err := json.Marshal(messages)
	if err != nil {
		return nil, &ConversionError{
			Message:  "failed to marshal input: " + err.Error(),
			Provider: "Responses",
		}
	}

	resultJSON, err := callRustFunction(fnResponsesToLingua, string(jsonBytes))
	if err != nil {
		return nil, &ConversionError{
			Message:  err.Error(),
			Provider: "Responses",
		}
	}

	var result []map[string]any
	err = json.Unmarshal([]byte(resultJSON), &result)
	if err != nil {
		return nil, &ConversionError{
			Message:  "failed to unmarshal result: " + err.Error(),
			Provider: "Responses",
		}
	}

	return result, nil
}

// LinguaToResponsesMessages converts Lingua messages to Responses API format.
//
//nolint:revive // Preserve exported name for backward compatibility
func LinguaToResponsesMessages(messages any) ([]map[string]any, error) {
	jsonBytes, err := json.Marshal(messages)
	if err != nil {
		return nil, &ConversionError{
			Message:  "failed to marshal input: " + err.Error(),
			Provider: "Responses",
		}
	}

	resultJSON, err := callRustFunction(fnLinguaToResponses, string(jsonBytes))
	if err != nil {
		return nil, &ConversionError{
			Message:  err.Error(),
			Provider: "Responses",
		}
	}

	var result []map[string]any
	err = json.Unmarshal([]byte(resultJSON), &result)
	if err != nil {
		return nil, &ConversionError{
			Message:  "failed to unmarshal result: " + err.Error(),
			Provider: "Responses",
		}
	}

	return result, nil
}

// ============================================================================
// Anthropic Conversions
// ============================================================================

// AnthropicMessagesToLingua converts Anthropic messages to Lingua format.
func AnthropicMessagesToLingua(messages any) ([]map[string]any, error) {
	jsonBytes, err := json.Marshal(messages)
	if err != nil {
		return nil, &ConversionError{
			Message:  "failed to marshal input: " + err.Error(),
			Provider: "Anthropic",
		}
	}

	resultJSON, err := callRustFunction(fnAnthropicToLingua, string(jsonBytes))
	if err != nil {
		return nil, &ConversionError{
			Message:  err.Error(),
			Provider: "Anthropic",
		}
	}

	var result []map[string]any
	err = json.Unmarshal([]byte(resultJSON), &result)
	if err != nil {
		return nil, &ConversionError{
			Message:  "failed to unmarshal result: " + err.Error(),
			Provider: "Anthropic",
		}
	}

	return result, nil
}

// LinguaToAnthropicMessages converts Lingua messages to Anthropic format.
//
//nolint:revive // Preserve exported name for backward compatibility
func LinguaToAnthropicMessages(messages any) ([]map[string]any, error) {
	jsonBytes, err := json.Marshal(messages)
	if err != nil {
		return nil, &ConversionError{
			Message:  "failed to marshal input: " + err.Error(),
			Provider: "Anthropic",
		}
	}

	resultJSON, err := callRustFunction(fnLinguaToAnthropic, string(jsonBytes))
	if err != nil {
		return nil, &ConversionError{
			Message:  err.Error(),
			Provider: "Anthropic",
		}
	}

	var result []map[string]any
	err = json.Unmarshal([]byte(resultJSON), &result)
	if err != nil {
		return nil, &ConversionError{
			Message:  "failed to unmarshal result: " + err.Error(),
			Provider: "Anthropic",
		}
	}

	return result, nil
}

// ============================================================================
// Processing Functions
// ============================================================================

// DeduplicateMessages removes duplicate messages based on role and content.
func DeduplicateMessages(messages any) ([]map[string]any, error) {
	jsonBytes, err := json.Marshal(messages)
	if err != nil {
		return nil, &ConversionError{
			Message: "failed to marshal input: " + err.Error(),
		}
	}

	resultJSON, err := callRustFunction(fnDeduplicateMessages, string(jsonBytes))
	if err != nil {
		return nil, &ConversionError{
			Message: err.Error(),
		}
	}

	var result []map[string]any
	err = json.Unmarshal([]byte(resultJSON), &result)
	if err != nil {
		return nil, &ConversionError{
			Message: "failed to unmarshal result: " + err.Error(),
		}
	}

	return result, nil
}

// ImportMessagesFromSpans extracts messages from spans by attempting multiple provider format conversions.
func ImportMessagesFromSpans(spans any) ([]map[string]any, error) {
	jsonBytes, err := json.Marshal(spans)
	if err != nil {
		return nil, &ConversionError{
			Message: "failed to marshal input: " + err.Error(),
		}
	}

	resultJSON, err := callRustFunction(fnImportMessagesFromSpans, string(jsonBytes))
	if err != nil {
		return nil, &ConversionError{
			Message: err.Error(),
		}
	}

	var result []map[string]any
	err = json.Unmarshal([]byte(resultJSON), &result)
	if err != nil {
		return nil, &ConversionError{
			Message: "failed to unmarshal result: " + err.Error(),
		}
	}

	return result, nil
}

// ============================================================================
// Validation Functions
// ============================================================================

// ValidateChatCompletionsRequest validates a JSON string as a Chat Completions request.
func ValidateChatCompletionsRequest(jsonStr string) (map[string]any, error) {
	resultJSON, err := callRustFunction(fnValidateChatCompletionsRequest, jsonStr)
	if err != nil {
		return nil, err
	}

	var result map[string]any
	if unmarshalErr := json.Unmarshal([]byte(resultJSON), &result); unmarshalErr != nil {
		return nil, errors.New("failed to unmarshal result: " + unmarshalErr.Error())
	}

	return result, nil
}

// ValidateChatCompletionsResponse validates a JSON string as a Chat Completions response.
func ValidateChatCompletionsResponse(jsonStr string) (map[string]any, error) {
	resultJSON, err := callRustFunction(fnValidateChatCompletionsResponse, jsonStr)
	if err != nil {
		return nil, err
	}

	var result map[string]any
	if unmarshalErr := json.Unmarshal([]byte(resultJSON), &result); unmarshalErr != nil {
		return nil, errors.New("failed to unmarshal result: " + unmarshalErr.Error())
	}

	return result, nil
}

// ValidateResponsesRequest validates a JSON string as a Responses API request.
func ValidateResponsesRequest(jsonStr string) (map[string]any, error) {
	resultJSON, err := callRustFunction(fnValidateResponsesRequest, jsonStr)
	if err != nil {
		return nil, err
	}

	var result map[string]any
	if unmarshalErr := json.Unmarshal([]byte(resultJSON), &result); unmarshalErr != nil {
		return nil, errors.New("failed to unmarshal result: " + unmarshalErr.Error())
	}

	return result, nil
}

// ValidateResponsesResponse validates a JSON string as a Responses API response.
func ValidateResponsesResponse(jsonStr string) (map[string]any, error) {
	resultJSON, err := callRustFunction(fnValidateResponsesResponse, jsonStr)
	if err != nil {
		return nil, err
	}

	var result map[string]any
	if unmarshalErr := json.Unmarshal([]byte(resultJSON), &result); unmarshalErr != nil {
		return nil, errors.New("failed to unmarshal result: " + unmarshalErr.Error())
	}

	return result, nil
}

// ValidateAnthropicRequest validates a JSON string as an Anthropic request.
func ValidateAnthropicRequest(jsonStr string) (map[string]any, error) {
	resultJSON, err := callRustFunction(fnValidateAnthropicRequest, jsonStr)
	if err != nil {
		return nil, err
	}

	var result map[string]any
	if unmarshalErr := json.Unmarshal([]byte(resultJSON), &result); unmarshalErr != nil {
		return nil, errors.New("failed to unmarshal result: " + unmarshalErr.Error())
	}

	return result, nil
}

// ValidateAnthropicResponse validates a JSON string as an Anthropic response.
func ValidateAnthropicResponse(jsonStr string) (map[string]any, error) {
	resultJSON, err := callRustFunction(fnValidateAnthropicResponse, jsonStr)
	if err != nil {
		return nil, err
	}

	var result map[string]any
	if unmarshalErr := json.Unmarshal([]byte(resultJSON), &result); unmarshalErr != nil {
		return nil, errors.New("failed to unmarshal result: " + unmarshalErr.Error())
	}

	return result, nil
}
