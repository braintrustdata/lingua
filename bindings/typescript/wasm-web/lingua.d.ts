/* tslint:disable */
/* eslint-disable */
/**
 * Convert array of Chat Completions messages to Lingua Messages
 */
export function chat_completions_messages_to_lingua(value: any): any;
/**
 * Convert array of Lingua Messages to Chat Completions messages
 */
export function lingua_to_chat_completions_messages(value: any): any;
/**
 * Convert array of Responses API messages to Lingua Messages
 */
export function responses_messages_to_lingua(value: any): any;
/**
 * Convert array of Lingua Messages to Responses API messages
 */
export function lingua_to_responses_messages(value: any): any;
/**
 * Convert array of Anthropic messages to Lingua Messages
 */
export function anthropic_messages_to_lingua(value: any): any;
/**
 * Convert array of Lingua Messages to Anthropic messages
 */
export function lingua_to_anthropic_messages(value: any): any;
/**
 * Deduplicate messages based on role and content
 */
export function deduplicate_messages(value: any): any;
/**
 * Import messages from spans
 */
export function import_messages_from_spans(value: any): any;
/**
 * Import and deduplicate messages from spans in a single operation
 */
export function import_and_deduplicate_messages(value: any): any;
/**
 * Validate a JSON string as a Chat Completions request
 */
export function validate_chat_completions_request(json: string): any;
/**
 * Validate a JSON string as a Chat Completions response
 */
export function validate_chat_completions_response(json: string): any;
/**
 * Validate a JSON string as a Responses API request
 */
export function validate_responses_request(json: string): any;
/**
 * Validate a JSON string as a Responses API response
 */
export function validate_responses_response(json: string): any;
/**
 * Validate a JSON string as an OpenAI request
 * @deprecated Use validate_chat_completions_request instead
 */
export function validate_openai_request(json: string): any;
/**
 * Validate a JSON string as an OpenAI response
 * @deprecated Use validate_chat_completions_response instead
 */
export function validate_openai_response(json: string): any;
/**
 * Validate a JSON string as an Anthropic request
 */
export function validate_anthropic_request(json: string): any;
/**
 * Validate a JSON string as an Anthropic response
 */
export function validate_anthropic_response(json: string): any;
/**
 * Validate a JSON string as a Google request (not supported - protobuf types)
 */
export function validate_google_request(json: string): any;
/**
 * Validate a JSON string as a Google response (not supported - protobuf types)
 */
export function validate_google_response(json: string): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly chat_completions_messages_to_lingua: (a: number, b: number) => void;
  readonly lingua_to_chat_completions_messages: (a: number, b: number) => void;
  readonly responses_messages_to_lingua: (a: number, b: number) => void;
  readonly lingua_to_responses_messages: (a: number, b: number) => void;
  readonly anthropic_messages_to_lingua: (a: number, b: number) => void;
  readonly lingua_to_anthropic_messages: (a: number, b: number) => void;
  readonly deduplicate_messages: (a: number, b: number) => void;
  readonly import_messages_from_spans: (a: number, b: number) => void;
  readonly import_and_deduplicate_messages: (a: number, b: number) => void;
  readonly validate_chat_completions_request: (a: number, b: number, c: number) => void;
  readonly validate_chat_completions_response: (a: number, b: number, c: number) => void;
  readonly validate_responses_request: (a: number, b: number, c: number) => void;
  readonly validate_responses_response: (a: number, b: number, c: number) => void;
  readonly validate_anthropic_request: (a: number, b: number, c: number) => void;
  readonly validate_anthropic_response: (a: number, b: number, c: number) => void;
  readonly validate_google_request: (a: number, b: number, c: number) => void;
  readonly validate_openai_request: (a: number, b: number, c: number) => void;
  readonly validate_openai_response: (a: number, b: number, c: number) => void;
  readonly validate_google_response: (a: number, b: number, c: number) => void;
  readonly __wbindgen_export_0: (a: number, b: number) => number;
  readonly __wbindgen_export_1: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_2: (a: number) => void;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
