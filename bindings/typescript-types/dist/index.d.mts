/**
 * Provider options - matching AI SDK Message format
 */
type ProviderOptions = Record<string, any>;

/**
 * Reusable text content part for tagged unions
 */
type TextContentPart = {
    text: string;
    encrypted_content?: string;
    provider_options?: ProviderOptions;
};

type ToolCallArguments = {
    "type": "valid";
    "value": Record<string, unknown>;
} | {
    "type": "invalid";
    "value": string;
};

/**
 * Assistant content parts - text, file, reasoning, tool calls, and tool results allowed
 */
type AssistantContentPart = {
    "type": "text";
} & TextContentPart | {
    "type": "file";
    data: string | Uint8Array | ArrayBuffer | Buffer | URL;
    filename?: string;
    media_type: string;
    provider_options?: ProviderOptions;
} | {
    "type": "reasoning";
    text: string;
    /**
     * Providers will occasionally return encrypted content for reasoning parts which can
     * be useful when you send a follow up message.
     */
    encrypted_content?: string;
} | {
    "type": "tool_call";
    tool_call_id: string;
    tool_name: string;
    arguments: ToolCallArguments;
    encrypted_content?: string;
    provider_options?: ProviderOptions;
    provider_executed?: boolean;
} | {
    "type": "tool_result";
    tool_call_id: string;
    tool_name: string;
    output: unknown;
    provider_options?: ProviderOptions;
};

type AssistantContent = string | Array<AssistantContentPart>;

/**
 * Reusable tool result content part for tagged unions
 */
type ToolResultContentPart = {
    tool_call_id: string;
    tool_name: string;
    output: any;
    provider_options?: ProviderOptions;
};

/**
 * Tool content parts - only tool results allowed
 */
type ToolContentPart = {
    "type": "tool_result";
} & ToolResultContentPart;

/**
 * User content parts - text, image, and file parts allowed
 */
type UserContentPart = {
    "type": "text";
} & TextContentPart | {
    "type": "image";
    image: string | Uint8Array | ArrayBuffer | Buffer | URL;
    media_type?: string;
    provider_options?: ProviderOptions;
} | {
    "type": "file";
    data: string | Uint8Array | ArrayBuffer | Buffer | URL;
    filename?: string;
    media_type: string;
    provider_options?: ProviderOptions;
};

type UserContent = string | Array<UserContentPart>;

type Message = {
    "role": "system";
    content: UserContent;
} | {
    "role": "developer";
    content: UserContent;
} | {
    "role": "user";
    content: UserContent;
} | {
    "role": "assistant";
    content: AssistantContent;
    id?: string;
} | {
    "role": "tool";
    content: Array<ToolContentPart>;
};

/**
 * Provider metadata
 */
type ProviderMetadata = Record<string, unknown>;

/**
 * Generated file content part - matching AI SDK GeneratedFile
 */
type GeneratedFileContentPart = {
    file: string | Uint8Array | ArrayBuffer | Buffer | URL;
    provider_metadata?: ProviderMetadata;
};

/**
 * Source content part - matching AI SDK Source type
 */
type SourceContentPart = {
    "source_type": "url";
    id: string;
    url: string;
    title?: string;
    provider_metadata?: ProviderMetadata;
} | {
    "source_type": "document";
    id: string;
    media_type: string;
    title: string;
    filename?: string;
    provider_metadata?: ProviderMetadata;
};

/**
 * Source type enum - matches AI SDK Source sourceType
 */
type SourceType$1 = "url" | "document";

/**
 * Tool call content part for response messages
 */
type ToolCallContentPart = {
    tool_call_id: string;
    tool_name: string;
    input: any;
    provider_executed?: boolean;
    provider_metadata?: ProviderMetadata;
};

/**
 * Tool error content part for response messages
 */
type ToolErrorContentPart = {
    tool_call_id: string;
    tool_name: string;
    error: string;
    provider_metadata?: ProviderMetadata;
};

/**
 * Tool result content part for response messages
 */
type ToolResultResponsePart = {
    tool_call_id: string;
    tool_name: string;
    output: any;
    provider_metadata?: ProviderMetadata;
};

/**
 * Indicates which field is the canonical source of truth for reasoning configuration.
 *
 * When converting between providers, both `effort` and `budget_tokens` are always populated
 * (one derived from the other). This field indicates which was the original source.
 */
type ReasoningCanonical = "effort" | "budget_tokens";

/**
 * Reasoning effort level (portable across providers).
 */
type ReasoningEffort = "low" | "medium" | "high";

/**
 * Summary mode for reasoning output.
 */
type SummaryMode = "None" | "Auto" | "Detailed";

/**
 * Configuration for extended thinking / reasoning capabilities.
 *
 * Both `effort` and `budget_tokens` are always populated when reasoning is enabled.
 * The `canonical` field indicates which was the original source of truth:
 * - `Effort`: From OpenAI (effort is canonical, budget_tokens derived)
 * - `BudgetTokens`: From Anthropic/Google (budget_tokens is canonical, effort derived)
 */
type ReasoningConfig = {
    /**
     * Whether reasoning/thinking is enabled.
     */
    enabled: boolean | null;
    /**
     * Reasoning effort level (low/medium/high).
     * Always populated when enabled. Used by OpenAI Chat/Responses API.
     */
    effort: ReasoningEffort | null;
    /**
     * Token budget for thinking.
     * Always populated when enabled. Used by Anthropic/Google.
     */
    budget_tokens: bigint | null;
    /**
     * Which field is the canonical source of truth.
     * Indicates whether `effort` or `budget_tokens` was the original value.
     */
    canonical: ReasoningCanonical | null;
    /**
     * Summary mode for reasoning output.
     * Maps to OpenAI Responses API's `reasoning.summary` field.
     */
    summary: SummaryMode | null;
};

/**
 * JSON schema configuration for structured output.
 */
type JsonSchemaConfig = {
    /**
     * Schema name (required by OpenAI)
     */
    name: string;
    /**
     * The JSON schema definition
     */
    schema: Record<string, unknown>;
    /**
     * Whether to enable strict schema validation
     */
    strict: boolean | null;
    /**
     * Human-readable description of the schema
     */
    description: string | null;
};

/**
 * Response format type (portable across providers).
 */
type ResponseFormatType = "Text" | "JsonObject" | "JsonSchema";

/**
 * Response format configuration for structured output.
 *
 * Provider mapping:
 * - OpenAI Chat: `{ type: "text" | "json_object" | "json_schema", json_schema? }`
 * - OpenAI Responses: nested under `text.format`
 * - Google: `response_mime_type` + `response_schema`
 * - Anthropic: `{ type: "json_schema", schema, name?, strict?, description? }`
 */
type ResponseFormatConfig = {
    /**
     * Output format type
     */
    format_type: ResponseFormatType | null;
    /**
     * JSON schema configuration (when format_type = JsonSchema)
     */
    json_schema: JsonSchemaConfig | null;
};

/**
 * Canonical token budget for request generation limits.
 *
 * This uses mutually-exclusive variants to avoid invalid combinations like
 * setting both "output" and "total" token limits at the same time.
 */
type TokenBudget = {
    "type": "output_tokens";
    "tokens": bigint;
} | {
    "type": "total_tokens";
    "tokens": bigint;
};

/**
 * Tool selection mode (portable across providers).
 */
type ToolChoiceMode = "Auto" | "None" | "Required" | "Tool";

/**
 * Tool selection strategy configuration.
 *
 * Uses canonical fields (`mode`, `tool_name`) for cross-provider conversion.
 *
 * Provider mapping:
 * - OpenAI Chat: `"auto"` | `"none"` | `"required"` | `{ type: "function", function: { name } }`
 * - OpenAI Responses: `"auto"` | `{ type: "function", name }`
 * - Anthropic: `{ type: "auto" | "any" | "none" | "tool", name?, disable_parallel_tool_use? }`
 */
type ToolChoiceConfig = {
    /**
     * Selection mode - the semantic intent of the tool choice
     */
    mode: ToolChoiceMode | null;
    /**
     * Specific tool name (when mode = Tool)
     */
    tool_name: string | null;
};

/**
 * Provider identity for built-in tool passthrough.
 */
type BuiltinToolProvider = "anthropic" | "responses" | "google" | "converse";

/**
 * A tool definition in universal format.
 *
 * This provides a typed representation that normalizes the different tool formats
 * across providers (Anthropic, OpenAI Chat, OpenAI Responses API, etc.).
 */
type UniversalTool = {
    /**
     * Tool name (required for all tool types)
     */
    name: string;
    /**
     * Tool description
     */
    description: string | null;
    /**
     * Parameters/input schema (JSON Schema)
     */
    parameters: Record<string, unknown> | null;
    /**
     * Whether to enforce strict schema validation (OpenAI Responses API)
     */
    strict: boolean | null;
} & ({
    "kind": "function";
} | {
    "kind": "custom";
    /**
     * Optional input format for custom tools (e.g. text/grammar config)
     */
    format: Record<string, unknown> | null;
} | {
    "kind": "builtin";
    /**
     * Provider identifier for built-in tool provenance
     */
    provider: BuiltinToolProvider;
    /**
     * Original type name (e.g., "bash_20250124", "code_interpreter")
     */
    builtin_type: string;
    /**
     * Provider-specific configuration
     */
    config: Record<string, unknown> | null;
});

/**
 * Common request parameters across providers.
 *
 * Uses canonical names - adapters handle mapping to provider-specific names.
 * Provider-specific fields without canonical mappings are stored in `extras`.
 */
type UniversalParams = {
    /**
     * Controls randomness: 0 = deterministic, 2 = maximum randomness.
     *
     * **Providers:** OpenAI, Anthropic, Google (`generationConfig.temperature`), Bedrock (`inferenceConfig.temperature`)
     */
    temperature: number | null;
    /**
     * Nucleus sampling: only consider tokens with cumulative probability ≤ top_p.
     *
     * **Providers:** OpenAI, Anthropic, Google (`generationConfig.topP`), Bedrock (`inferenceConfig.topP`)
     */
    top_p: number | null;
    /**
     * Only sample from the top K most likely tokens.
     *
     * **Providers:** Anthropic, Google (`generationConfig.topK`)
     */
    top_k: bigint | null;
    /**
     * Random seed for deterministic generation.
     *
     * **Providers:** OpenAI
     */
    seed: bigint | null;
    /**
     * Penalize tokens based on whether they've appeared at all (-2.0 to 2.0).
     *
     * **Providers:** OpenAI
     */
    presence_penalty: number | null;
    /**
     * Penalize tokens based on how often they've appeared (-2.0 to 2.0).
     *
     * **Providers:** OpenAI
     */
    frequency_penalty: number | null;
    /**
     * Generation token budget.
     *
     * **Providers:** OpenAI (`max_tokens`/`max_completion_tokens`/`max_output_tokens`),
     * Anthropic (`max_tokens`), Google (`generationConfig.maxOutputTokens`),
     * Bedrock (`inferenceConfig.maxTokens`) all map to `OutputTokens`.
     */
    token_budget: TokenBudget | null;
    /**
     * Sequences that stop generation when encountered.
     *
     * **Providers:** OpenAI, Anthropic (`stop_sequences`), Google (`generationConfig.stopSequences`), Bedrock (`inferenceConfig.stopSequences`)
     */
    stop: Array<string> | null;
    /**
     * Return log probabilities of output tokens.
     *
     * **Providers:** OpenAI
     */
    logprobs: boolean | null;
    /**
     * Number of most likely tokens to return log probabilities for (0-20).
     *
     * **Providers:** OpenAI
     */
    top_logprobs: bigint | null;
    /**
     * Tool/function definitions the model can call.
     *
     * **Providers:** OpenAI, Anthropic, Google (`tools[].functionDeclarations`), Bedrock (`toolConfig.tools[].toolSpec`)
     */
    tools: Array<UniversalTool> | null;
    /**
     * How the model should choose which tool to call.
     *
     * **Providers:** OpenAI, Anthropic
     */
    tool_choice: ToolChoiceConfig | null;
    /**
     * Allow multiple tool calls in a single response.
     *
     * **Providers:** OpenAI, Anthropic (`tool_choice.disable_parallel_tool_use`)
     */
    parallel_tool_calls: boolean | null;
    /**
     * Constrain output format (text, JSON, or JSON schema).
     *
     * **Providers:** OpenAI, Anthropic (`output_format`)
     */
    response_format: ResponseFormatConfig | null;
    /**
     * Enable extended thinking / chain-of-thought reasoning.
     *
     * **Providers:** OpenAI (`reasoning_effort`), Anthropic (`thinking`), Google (`generationConfig.thinkingConfig`), Bedrock (`additionalModelRequestFields.thinking`)
     */
    reasoning: ReasoningConfig | null;
    /**
     * Key-value metadata attached to the request.
     *
     * **Providers:** OpenAI, Anthropic (only `user_id`)
     */
    metadata: Record<string, unknown> | null;
    /**
     * Store the completion for later use in fine-tuning or evals.
     *
     * **Providers:** OpenAI
     */
    store: boolean | null;
    /**
     * Request priority tier (e.g., "auto", "default").
     *
     * **Providers:** OpenAI, Anthropic
     */
    service_tier: string | null;
    /**
     * Stream the response as server-sent events.
     *
     * **Providers:** OpenAI, Anthropic
     */
    stream: boolean | null;
};

/**
 * Universal request envelope for LLM API calls.
 *
 * This type captures the common structure across all provider request formats.
 */
type UniversalRequest = {
    /**
     * Model identifier (may be None for providers that use endpoint-based model selection)
     */
    model: string | null;
    /**
     * Conversation messages in universal format
     */
    messages: Array<Message>;
    /**
     * Request parameters (canonical fields + provider-specific extras)
     */
    params: UniversalParams;
};

/**
 * Represents the API format/protocol used by an LLM provider.
 *
 * This enum is the single source of truth for provider formats across the ecosystem.
 * When adding a new provider format:
 * 1. Add a variant here
 * 2. Update detection heuristics in `processing/detect.rs`
 * 3. Add conversion logic in `providers/<name>/convert.rs` if needed
 */
type ProviderFormat = "openai" | "anthropic" | "google" | "mistral" | "converse" | "responses" | "unknown";

/**
 * Classification of tool types.
 */
type UniversalToolType = {
    "kind": "function";
} | {
    "kind": "custom";
    /**
     * Optional input format for custom tools (e.g. text/grammar config)
     */
    format: Record<string, unknown> | null;
} | {
    "kind": "builtin";
    /**
     * Provider identifier for built-in tool provenance
     */
    provider: BuiltinToolProvider;
    /**
     * Original type name (e.g., "bash_20250124", "code_interpreter")
     */
    builtin_type: string;
    /**
     * Provider-specific configuration
     */
    config: Record<string, unknown> | null;
};

/**
 * Data about a previous audio response from the model.
 * [Learn more](https://platform.openai.com/docs/guides/audio).
 */
type ChatCompletionRequestMessageAudio = {
    /**
     * Unique identifier for a previous audio response from the model.
     */
    id: string;
};

/**
 * The format of the encoded audio data. Currently supports "wav" and "mp3".
 *
 *
 * The format of the audio data. Currently supported formats are `mp3` and
 * `wav`.
 */
type InputAudioFormat = "mp3" | "wav";

type ArrayOfContentPartInputAudio = {
    /**
     * Base64 encoded audio data.
     */
    data: string;
    /**
     * The format of the encoded audio data. Currently supports "wav" and "mp3".
     */
    format: InputAudioFormat;
};

type File = {
    /**
     * The base64 encoded file data, used when passing the file to the model
     * as a string.
     */
    file_data: string | null;
    /**
     * The ID of an uploaded file to use as input.
     */
    file_id: string | null;
    /**
     * The name of the file, used when passing the file to the model as a
     * string.
     */
    filename: string | null;
};

/**
 * Specifies the detail level of the image. Learn more in the [Vision
 * guide](https://platform.openai.com/docs/guides/vision#low-or-high-fidelity-image-understanding).
 *
 * The detail level of the image to be sent to the model. One of `high`, `low`, or `auto`.
 * Defaults to `auto`.
 */
type ImageDetail = "auto" | "high" | "low";

type ImageUrl = {
    /**
     * Specifies the detail level of the image. Learn more in the [Vision
     * guide](https://platform.openai.com/docs/guides/vision#low-or-high-fidelity-image-understanding).
     */
    detail: ImageDetail | null;
    /**
     * Either a URL of the image or the base64 encoded image data.
     */
    url: string;
};

/**
 * The type of the content part.
 *
 * The type of the content part. Always `input_audio`.
 *
 * The type of the content part. Always `file`.
 */
type PurpleType$1 = "file" | "image_url" | "input_audio" | "refusal" | "text";

/**
 * An array of content parts with a defined type. For developer messages, only type `text`
 * is supported.
 *
 * Learn about [text inputs](https://platform.openai.com/docs/guides/text-generation).
 *
 *
 * An array of content parts with a defined type. Supported options differ based on the
 * [model](https://platform.openai.com/docs/models) being used to generate the response. Can
 * contain text inputs.
 *
 * An array of content parts with a defined type. For system messages, only type `text` is
 * supported.
 *
 * An array of content parts with a defined type. For tool messages, only type `text` is
 * supported.
 *
 * An array of content parts with a defined type. Supported options differ based on the
 * [model](https://platform.openai.com/docs/models) being used to generate the response. Can
 * contain text, image, or audio inputs.
 *
 * Learn about [image inputs](https://platform.openai.com/docs/guides/vision).
 *
 *
 * Learn about [audio inputs](https://platform.openai.com/docs/guides/audio).
 *
 *
 * Learn about [file inputs](https://platform.openai.com/docs/guides/text) for text
 * generation.
 *
 *
 * An array of content parts with a defined type. Can be one or more of type `text`, or
 * exactly one of type `refusal`.
 */
type ChatCompletionRequestMessageContentPart = {
    /**
     * The text content.
     */
    text: string | null;
    /**
     * The type of the content part.
     *
     * The type of the content part. Always `input_audio`.
     *
     * The type of the content part. Always `file`.
     */
    type: PurpleType$1;
    image_url: ImageUrl | null;
    input_audio: ArrayOfContentPartInputAudio | null;
    file: File | null;
    /**
     * The refusal message generated by the model.
     */
    refusal: string | null;
};

type ChatCompletionRequestMessageContent = Array<ChatCompletionRequestMessageContentPart> | string;

/**
 * Deprecated and replaced by `tool_calls`. The name and arguments of a function that should
 * be called, as generated by the model.
 */
type ChatCompletionRequestMessageFunctionCall = {
    /**
     * The arguments to call the function with, as generated by the model in JSON format. Note
     * that the model does not always generate valid JSON, and may hallucinate parameters not
     * defined by your function schema. Validate the arguments in your code before calling your
     * function.
     */
    arguments: string;
    /**
     * The name of the function to call.
     */
    name: string;
};

/**
 * The role of the messages author, in this case `developer`.
 *
 * The role of the messages author, in this case `system`.
 *
 * The role of the messages author, in this case `user`.
 *
 * The role of the messages author, in this case `assistant`.
 *
 * The role of the messages author, in this case `tool`.
 *
 * The role of the messages author, in this case `function`.
 */
type ChatCompletionRequestMessageRole = "assistant" | "developer" | "function" | "system" | "tool" | "user";

/**
 * The function that the model called.
 */
type PurpleFunction = {
    /**
     * The arguments to call the function with, as generated by the model in JSON format. Note
     * that the model does not always generate valid JSON, and may hallucinate parameters not
     * defined by your function schema. Validate the arguments in your code before calling your
     * function.
     */
    arguments: string;
    /**
     * The name of the function to call.
     */
    name: string;
};

/**
 * The custom tool that the model called.
 */
type ToolCallCustom = {
    /**
     * The input for the custom tool call generated by the model.
     */
    input: string;
    /**
     * The name of the custom tool to call.
     */
    name: string;
};

/**
 * The type of the tool. Currently, only `function` is supported.
 *
 * The type of the tool. Always `custom`.
 *
 * The type of the custom tool. Always `custom`.
 */
type ToolType = "custom" | "function";

/**
 * The tool calls generated by the model, such as function calls.
 *
 * A call to a function tool created by the model.
 *
 *
 * A call to a custom tool created by the model.
 */
type ToolCall = {
    /**
     * The function that the model called.
     */
    function: PurpleFunction | null;
    /**
     * The ID of the tool call.
     */
    id: string;
    /**
     * The type of the tool. Currently, only `function` is supported.
     *
     * The type of the tool. Always `custom`.
     */
    type: ToolType;
    /**
     * The custom tool that the model called.
     */
    custom: ToolCallCustom | null;
};

/**
 * Developer-provided instructions that the model should follow, regardless of
 * messages sent by the user. With o1 models and newer, `developer` messages
 * replace the previous `system` messages.
 *
 *
 * Developer-provided instructions that the model should follow, regardless of
 * messages sent by the user. With o1 models and newer, use `developer` messages
 * for this purpose instead.
 *
 *
 * Messages sent by an end user, containing prompts or additional context
 * information.
 *
 *
 * Messages sent by the model in response to user messages.
 */
type ChatCompletionRequestMessage = {
    /**
     * The contents of the developer message.
     *
     * The contents of the system message.
     *
     * The contents of the user message.
     *
     *
     * The contents of the tool message.
     */
    content: ChatCompletionRequestMessageContent | null;
    /**
     * An optional name for the participant. Provides the model information to differentiate
     * between participants of the same role.
     *
     * The name of the function to call.
     */
    name: string | null;
    /**
     * The role of the messages author, in this case `developer`.
     *
     * The role of the messages author, in this case `system`.
     *
     * The role of the messages author, in this case `user`.
     *
     * The role of the messages author, in this case `assistant`.
     *
     * The role of the messages author, in this case `tool`.
     *
     * The role of the messages author, in this case `function`.
     */
    role: ChatCompletionRequestMessageRole;
    audio: ChatCompletionRequestMessageAudio | null;
    function_call: ChatCompletionRequestMessageFunctionCall | null;
    refusal: string | null;
    tool_calls: Array<ToolCall> | null;
    /**
     * Tool call that this message is responding to.
     */
    tool_call_id: string | null;
};

/**
 * The type of the output. Always 'logs'.
 *
 * The type of the output. Always 'image'.
 */
type IndigoType = "image" | "logs";

/**
 * The outputs generated by the code interpreter, such as logs or images.
 * Can be null if no outputs are available.
 *
 *
 * The logs output from the code interpreter.
 *
 *
 * The image output from the code interpreter.
 */
type CodeInterpreterOutput = {
    /**
     * The logs output from the code interpreter.
     */
    logs: string | null;
    /**
     * The type of the output. Always 'logs'.
     *
     * The type of the output. Always 'image'.
     */
    type: IndigoType;
    /**
     * The URL of the image output from the code interpreter.
     */
    url: string | null;
};

/**
 * Specifies the event type. For a click action, this property is
 * always set to `click`.
 *
 *
 * Specifies the event type. For a double click action, this property is
 * always set to `double_click`.
 *
 *
 * Specifies the event type. For a drag action, this property is
 * always set to `drag`.
 *
 *
 * Specifies the event type. For a keypress action, this property is
 * always set to `keypress`.
 *
 *
 * Specifies the event type. For a move action, this property is
 * always set to `move`.
 *
 *
 * Specifies the event type. For a screenshot action, this property is
 * always set to `screenshot`.
 *
 *
 * Specifies the event type. For a scroll action, this property is
 * always set to `scroll`.
 *
 *
 * Specifies the event type. For a type action, this property is
 * always set to `type`.
 *
 *
 * Specifies the event type. For a wait action, this property is
 * always set to `wait`.
 *
 *
 * The action type.
 *
 *
 * The type of the local shell action. Always `exec`.
 */
type ActionType = "click" | "double_click" | "drag" | "exec" | "find" | "keypress" | "move" | "open_page" | "screenshot" | "scroll" | "search" | "type" | "wait";

/**
 * Indicates which mouse button was pressed during the click. One of `left`, `right`,
 * `wheel`, `back`, or `forward`.
 */
type Button = "back" | "forward" | "left" | "right" | "wheel";

/**
 * A series of x/y coordinate pairs in the drag path.
 *
 *
 * An x/y coordinate pair, e.g. `{ x: 100, y: 200 }`.
 */
type Coordinate = {
    /**
     * The x-coordinate.
     */
    x: bigint;
    /**
     * The y-coordinate.
     */
    y: bigint;
};

/**
 * The type of source. Always `url`.
 */
type SourceType = "url";

/**
 * A source used in the search.
 */
type WebSearchSource = {
    /**
     * The type of source. Always `url`.
     */
    type: SourceType;
    /**
     * The URL of the source.
     */
    url: string;
};

/**
 * A click action.
 *
 *
 * A double click action.
 *
 *
 * A drag action.
 *
 *
 * A collection of keypresses the model would like to perform.
 *
 *
 * A mouse move action.
 *
 *
 * A screenshot action.
 *
 *
 * A scroll action.
 *
 *
 * An action to type in text.
 *
 *
 * A wait action.
 *
 *
 * An object describing the specific action taken in this web search call.
 * Includes details on how the model used the web (search, open_page, find).
 *
 *
 * Action type "search" - Performs a web search query.
 *
 *
 * Action type "open_page" - Opens a specific URL from search results.
 *
 *
 * Action type "find": Searches for a pattern within a loaded page.
 *
 *
 * Execute a shell command on the server.
 */
type ComputerAction = {
    /**
     * Indicates which mouse button was pressed during the click. One of `left`, `right`,
     * `wheel`, `back`, or `forward`.
     */
    button: Button | null;
    /**
     * Specifies the event type. For a click action, this property is
     * always set to `click`.
     *
     *
     * Specifies the event type. For a double click action, this property is
     * always set to `double_click`.
     *
     *
     * Specifies the event type. For a drag action, this property is
     * always set to `drag`.
     *
     *
     * Specifies the event type. For a keypress action, this property is
     * always set to `keypress`.
     *
     *
     * Specifies the event type. For a move action, this property is
     * always set to `move`.
     *
     *
     * Specifies the event type. For a screenshot action, this property is
     * always set to `screenshot`.
     *
     *
     * Specifies the event type. For a scroll action, this property is
     * always set to `scroll`.
     *
     *
     * Specifies the event type. For a type action, this property is
     * always set to `type`.
     *
     *
     * Specifies the event type. For a wait action, this property is
     * always set to `wait`.
     *
     *
     * The action type.
     *
     *
     * The type of the local shell action. Always `exec`.
     */
    type: ActionType;
    /**
     * The x-coordinate where the click occurred.
     *
     *
     * The x-coordinate where the double click occurred.
     *
     *
     * The x-coordinate to move to.
     *
     *
     * The x-coordinate where the scroll occurred.
     */
    x: bigint | null;
    /**
     * The y-coordinate where the click occurred.
     *
     *
     * The y-coordinate where the double click occurred.
     *
     *
     * The y-coordinate to move to.
     *
     *
     * The y-coordinate where the scroll occurred.
     */
    y: bigint | null;
    /**
     * An array of coordinates representing the path of the drag action. Coordinates will appear
     * as an array
     * of objects, eg
     * ```json
     * [
     * { "x": 100, "y": 200 },
     * { "x": 200, "y": 300 }
     * ]
     * ```
     */
    path: Array<Coordinate> | null;
    /**
     * The combination of keys the model is requesting to be pressed. This is an
     * array of strings, each representing a key.
     */
    keys: Array<string> | null;
    /**
     * The horizontal scroll distance.
     */
    scroll_x: bigint | null;
    /**
     * The vertical scroll distance.
     */
    scroll_y: bigint | null;
    /**
     * The text to type.
     */
    text: string | null;
    /**
     * The search query.
     */
    query: string | null;
    /**
     * The sources used in the search.
     */
    sources: Array<WebSearchSource> | null;
    /**
     * The URL opened by the model.
     *
     *
     * The URL of the page searched for the pattern.
     */
    url: string | null;
    /**
     * The pattern or text to search for within the page.
     */
    pattern: string | null;
    /**
     * The command to run.
     */
    command: Array<string> | null;
    /**
     * Environment variables to set for the command.
     */
    env: {
        [key in string]?: string;
    } | null;
    timeout_ms: bigint | null;
    user: string | null;
    working_directory: string | null;
};

/**
 * The safety checks reported by the API that have been acknowledged by the developer.
 *
 * A pending safety check for the computer call.
 */
type ComputerCallSafetyCheckParam = {
    code: string | null;
    /**
     * The ID of the pending safety check.
     */
    id: string;
    message: string | null;
};

/**
 * A pending safety check for the computer call.
 */
type ComputerToolCallSafetyCheck = {
    /**
     * The type of the pending safety check.
     */
    code: string;
    /**
     * The ID of the pending safety check.
     */
    id: string;
    /**
     * Details about the pending safety check.
     */
    message: string;
};

/**
 * The status of item. One of `in_progress`, `completed`, or
 * `incomplete`. Populated when items are returned via API.
 *
 *
 * The status of the message input. One of `in_progress`, `completed`, or
 * `incomplete`. Populated when input items are returned via API.
 *
 *
 * The status of the file search tool call. One of `in_progress`,
 * `searching`, `incomplete` or `failed`,
 *
 *
 * The status of the item. One of `in_progress`, `completed`, or
 * `incomplete`. Populated when items are returned via API.
 *
 *
 * The status of the message input. One of `in_progress`, `completed`, or `incomplete`.
 * Populated when input items are returned via API.
 *
 * The status of the item. One of `in_progress`, `completed`, or `incomplete`. Populated
 * when items are returned via API.
 *
 * The status of the web search tool call.
 *
 *
 * The status of the image generation call.
 *
 *
 * The status of the code interpreter tool call. Valid values are `in_progress`,
 * `completed`, `incomplete`, `interpreting`, and `failed`.
 *
 *
 * The status of the local shell call.
 *
 *
 * The status of the item. One of `in_progress`, `completed`, or `incomplete`.
 */
type FunctionCallItemStatus = "completed" | "failed" | "generating" | "in_progress" | "incomplete" | "interpreting" | "searching";

/**
 * The type of the file citation. Always `file_citation`.
 *
 * The type of the URL citation. Always `url_citation`.
 *
 * The type of the container file citation. Always `container_file_citation`.
 *
 * The type of the file path. Always `file_path`.
 */
type AnnotationTypeEnum = "container_file_citation" | "file_citation" | "file_path" | "url_citation";

/**
 * A citation to a file.
 *
 * A citation for a web resource used to generate a model response.
 *
 * A citation for a container file used to generate a model response.
 *
 * A path to a file.
 */
type Annotation = {
    /**
     * The ID of the file.
     *
     * The ID of the file.
     */
    file_id: string | null;
    /**
     * The filename of the file cited.
     *
     * The filename of the container file cited.
     */
    filename: string | null;
    /**
     * The index of the file in the list of files.
     *
     * The index of the file in the list of files.
     */
    index: bigint | null;
    /**
     * The type of the file citation. Always `file_citation`.
     *
     * The type of the URL citation. Always `url_citation`.
     *
     * The type of the container file citation. Always `container_file_citation`.
     *
     * The type of the file path. Always `file_path`.
     */
    type: AnnotationTypeEnum;
    /**
     * The index of the last character of the URL citation in the message.
     *
     * The index of the last character of the container file citation in the message.
     */
    end_index: bigint | null;
    /**
     * The index of the first character of the URL citation in the message.
     *
     * The index of the first character of the container file citation in the message.
     */
    start_index: bigint | null;
    /**
     * The title of the web resource.
     */
    title: string | null;
    /**
     * The URL of the web resource.
     */
    url: string | null;
    /**
     * The ID of the container file.
     */
    container_id: string | null;
};

type InputItemContentListInputAudio = {
    /**
     * Base64-encoded audio data.
     */
    data: string;
    /**
     * The format of the audio data. Currently supported formats are `mp3` and
     * `wav`.
     */
    format: InputAudioFormat;
};

/**
 * The type of the input item. Always `input_text`.
 *
 * The type of the input item. Always `input_image`.
 *
 * The type of the input item. Always `input_file`.
 *
 * The type of the input item. Always `input_audio`.
 *
 *
 * The type of the output text. Always `output_text`.
 *
 * The type of the refusal. Always `refusal`.
 *
 * The type of the reasoning text. Always `reasoning_text`.
 */
type InputItemContentListType = "input_audio" | "input_file" | "input_image" | "input_text" | "output_text" | "reasoning_text" | "refusal";

/**
 * The top log probability of a token.
 */
type TopLogProbability = {
    bytes: Array<bigint>;
    logprob: number;
    token: string;
};

/**
 * The log probability of a token.
 */
type LogProbability = {
    bytes: Array<bigint>;
    logprob: number;
    token: string;
    top_logprobs: Array<TopLogProbability>;
};

/**
 * A list of one or many input items to the model, containing different content
 * types.
 *
 *
 * A text input to the model.
 *
 * An image input to the model. Learn about [image
 * inputs](https://platform.openai.com/docs/guides/vision).
 *
 * A file input to the model.
 *
 * An audio input to the model.
 *
 *
 * A text output from the model.
 *
 * A refusal from the model.
 *
 * Reasoning text from the model.
 */
type InputContent = {
    /**
     * The text input to the model.
     *
     * The text output from the model.
     *
     * The reasoning text from the model.
     */
    text: string | null;
    /**
     * The type of the input item. Always `input_text`.
     *
     * The type of the input item. Always `input_image`.
     *
     * The type of the input item. Always `input_file`.
     *
     * The type of the input item. Always `input_audio`.
     *
     *
     * The type of the output text. Always `output_text`.
     *
     * The type of the refusal. Always `refusal`.
     *
     * The type of the reasoning text. Always `reasoning_text`.
     */
    type: InputItemContentListType;
    /**
     * The detail level of the image to be sent to the model. One of `high`, `low`, or `auto`.
     * Defaults to `auto`.
     */
    detail: ImageDetail | null;
    file_id: string | null;
    image_url: string | null;
    /**
     * The content of the file to be sent to the model.
     */
    file_data: string | null;
    /**
     * The URL of the file to be sent to the model.
     */
    file_url: string | null;
    /**
     * The name of the file to be sent to the model.
     */
    filename: string | null;
    input_audio: InputItemContentListInputAudio | null;
    /**
     * The annotations of the text output.
     */
    annotations: Array<Annotation> | null;
    logprobs: Array<LogProbability> | null;
    /**
     * The refusal explanation from the model.
     */
    refusal: string | null;
};

type InputItemContent = Array<InputContent> | string;

/**
 * The role of the message input. One of `user`, `assistant`, `system`, or
 * `developer`.
 *
 *
 * The role of the message input. One of `user`, `system`, or `developer`.
 *
 *
 * The role of the output message. Always `assistant`.
 */
type InputItemRole = "assistant" | "developer" | "system" | "user";

/**
 * The type of the message input. Always `message`.
 *
 *
 * The type of the message input. Always set to `message`.
 *
 *
 * The type of the output message. Always `message`.
 *
 *
 * The type of the file search tool call. Always `file_search_call`.
 *
 *
 * The type of the computer call. Always `computer_call`.
 *
 * The type of the computer tool call output. Always `computer_call_output`.
 *
 * The type of the web search tool call. Always `web_search_call`.
 *
 *
 * The type of the function tool call. Always `function_call`.
 *
 *
 * The type of the function tool call output. Always `function_call_output`.
 *
 * The type of the object. Always `reasoning`.
 *
 *
 * The type of the image generation call. Always `image_generation_call`.
 *
 *
 * The type of the code interpreter tool call. Always `code_interpreter_call`.
 *
 *
 * The type of the local shell call. Always `local_shell_call`.
 *
 *
 * The type of the local shell tool call output. Always `local_shell_call_output`.
 *
 *
 * The type of the item. Always `mcp_list_tools`.
 *
 *
 * The type of the item. Always `mcp_approval_request`.
 *
 *
 * The type of the item. Always `mcp_approval_response`.
 *
 *
 * The type of the item. Always `mcp_call`.
 *
 *
 * The type of the custom tool call output. Always `custom_tool_call_output`.
 *
 *
 * The type of the custom tool call. Always `custom_tool_call`.
 *
 *
 * The type of item to reference. Always `item_reference`.
 */
type InputItemType = "code_interpreter_call" | "computer_call" | "computer_call_output" | "custom_tool_call" | "custom_tool_call_output" | "file_search_call" | "function_call" | "function_call_output" | "image_generation_call" | "item_reference" | "local_shell_call" | "local_shell_call_output" | "mcp_approval_request" | "mcp_approval_response" | "mcp_call" | "mcp_list_tools" | "message" | "reasoning" | "web_search_call";

/**
 * A tool available on an MCP server.
 */
type McpListToolsTool = {
    annotations: unknown;
    description: string | null;
    /**
     * The JSON schema describing the tool's input.
     */
    input_schema: unknown;
    /**
     * The name of the tool.
     */
    name: string;
};

type VectorStoreFileAttribute = boolean | number | string;

/**
 * The results of the file search tool call.
 */
type Result = {
    attributes: {
        [key in string]?: VectorStoreFileAttribute;
    } | null;
    /**
     * The unique ID of the file.
     */
    file_id: string | null;
    /**
     * The name of the file.
     */
    filename: string | null;
    /**
     * The relevance score of the file - a value between 0 and 1.
     */
    score: number | null;
    /**
     * The text that was retrieved from the file.
     */
    text: string | null;
};

/**
 * The type of the object. Always `summary_text`.
 */
type SummaryType = "summary_text";

/**
 * A summary text from the model.
 */
type SummaryText = {
    /**
     * A summary of the reasoning output from the model so far.
     */
    text: string;
    /**
     * The type of the object. Always `summary_text`.
     */
    type: SummaryType;
};

/**
 * A list of one or many input items to the model, containing
 * different content types.
 *
 *
 * A message input to the model with a role indicating instruction following
 * hierarchy. Instructions given with the `developer` or `system` role take
 * precedence over instructions given with the `user` role. Messages with the
 * `assistant` role are presumed to have been generated by the model in previous
 * interactions.
 *
 *
 * An item representing part of the context for the response to be
 * generated by the model. Can contain text, images, and audio inputs,
 * as well as previous assistant responses and tool call outputs.
 *
 *
 * Content item used to generate a response.
 *
 *
 * A message input to the model with a role indicating instruction following
 * hierarchy. Instructions given with the `developer` or `system` role take
 * precedence over instructions given with the `user` role.
 *
 *
 * An output message from the model.
 *
 *
 * The results of a file search tool call. See the
 * [file search guide](https://platform.openai.com/docs/guides/tools-file-search) for more
 * information.
 *
 *
 * A tool call to a computer use tool. See the
 * [computer use guide](https://platform.openai.com/docs/guides/tools-computer-use) for more
 * information.
 *
 *
 * The output of a computer tool call.
 *
 * The results of a web search tool call. See the
 * [web search guide](https://platform.openai.com/docs/guides/tools-web-search) for more
 * information.
 *
 *
 * A tool call to run a function. See the
 * [function calling guide](https://platform.openai.com/docs/guides/function-calling) for
 * more information.
 *
 *
 * The output of a function tool call.
 *
 * A description of the chain of thought used by a reasoning model while generating
 * a response. Be sure to include these items in your `input` to the Responses API
 * for subsequent turns of a conversation if you are manually
 * [managing context](https://platform.openai.com/docs/guides/conversation-state).
 *
 *
 * An image generation request made by the model.
 *
 *
 * A tool call to run code.
 *
 *
 * A tool call to run a command on the local shell.
 *
 *
 * The output of a local shell tool call.
 *
 *
 * A list of tools available on an MCP server.
 *
 *
 * A request for human approval of a tool invocation.
 *
 *
 * A response to an MCP approval request.
 *
 *
 * An invocation of a tool on an MCP server.
 *
 *
 * The output of a custom tool call from your code, being sent back to the model.
 *
 *
 * A call to a custom tool created by the model.
 *
 *
 * An internal identifier for an item to reference.
 */
type InputItem = {
    /**
     * Text, image, or audio input to the model, used to generate a response.
     * Can also contain previous assistant responses.
     *
     *
     * The content of the output message.
     *
     *
     * Reasoning text content.
     */
    content: InputItemContent | null;
    /**
     * The role of the message input. One of `user`, `assistant`, `system`, or
     * `developer`.
     *
     *
     * The role of the message input. One of `user`, `system`, or `developer`.
     *
     *
     * The role of the output message. Always `assistant`.
     */
    role: InputItemRole | null;
    /**
     * The type of the message input. Always `message`.
     *
     *
     * The type of the message input. Always set to `message`.
     *
     *
     * The type of the output message. Always `message`.
     *
     *
     * The type of the file search tool call. Always `file_search_call`.
     *
     *
     * The type of the computer call. Always `computer_call`.
     *
     * The type of the computer tool call output. Always `computer_call_output`.
     *
     * The type of the web search tool call. Always `web_search_call`.
     *
     *
     * The type of the function tool call. Always `function_call`.
     *
     *
     * The type of the function tool call output. Always `function_call_output`.
     *
     * The type of the object. Always `reasoning`.
     *
     *
     * The type of the image generation call. Always `image_generation_call`.
     *
     *
     * The type of the code interpreter tool call. Always `code_interpreter_call`.
     *
     *
     * The type of the local shell call. Always `local_shell_call`.
     *
     *
     * The type of the local shell tool call output. Always `local_shell_call_output`.
     *
     *
     * The type of the item. Always `mcp_list_tools`.
     *
     *
     * The type of the item. Always `mcp_approval_request`.
     *
     *
     * The type of the item. Always `mcp_approval_response`.
     *
     *
     * The type of the item. Always `mcp_call`.
     *
     *
     * The type of the custom tool call output. Always `custom_tool_call_output`.
     *
     *
     * The type of the custom tool call. Always `custom_tool_call`.
     */
    type: InputItemType | null;
    /**
     * The status of item. One of `in_progress`, `completed`, or
     * `incomplete`. Populated when items are returned via API.
     *
     *
     * The status of the message input. One of `in_progress`, `completed`, or
     * `incomplete`. Populated when input items are returned via API.
     *
     *
     * The status of the file search tool call. One of `in_progress`,
     * `searching`, `incomplete` or `failed`,
     *
     *
     * The status of the item. One of `in_progress`, `completed`, or
     * `incomplete`. Populated when items are returned via API.
     *
     *
     * The status of the web search tool call.
     *
     *
     * The status of the image generation call.
     *
     *
     * The status of the code interpreter tool call. Valid values are `in_progress`,
     * `completed`, `incomplete`, `interpreting`, and `failed`.
     *
     *
     * The status of the local shell call.
     */
    status: FunctionCallItemStatus | null;
    /**
     * The unique ID of the output message.
     *
     *
     * The unique ID of the file search tool call.
     *
     *
     * The unique ID of the computer call.
     *
     * The unique ID of the web search tool call.
     *
     *
     * The unique ID of the function tool call.
     *
     *
     * The unique identifier of the reasoning content.
     *
     *
     * The unique ID of the image generation call.
     *
     *
     * The unique ID of the code interpreter tool call.
     *
     *
     * The unique ID of the local shell call.
     *
     *
     * The unique ID of the local shell tool call generated by the model.
     *
     *
     * The unique ID of the list.
     *
     *
     * The unique ID of the approval request.
     *
     *
     * The unique ID of the tool call.
     *
     *
     * The unique ID of the custom tool call output in the OpenAI platform.
     *
     *
     * The unique ID of the custom tool call in the OpenAI platform.
     *
     *
     * The ID of the item to reference.
     */
    id: string | null;
    /**
     * The queries used to search for files.
     */
    queries: Array<string> | null;
    results: Array<Result> | null;
    /**
     * An object describing the specific action taken in this web search call.
     * Includes details on how the model used the web (search, open_page, find).
     */
    action: ComputerAction | null;
    /**
     * An identifier used when responding to the tool call with output.
     *
     *
     * The ID of the computer tool call that produced the output.
     *
     * The unique ID of the function tool call generated by the model.
     *
     *
     * The unique ID of the function tool call generated by the model.
     *
     * The unique ID of the local shell tool call generated by the model.
     *
     *
     * The call ID, used to map this custom tool call output to a custom tool call.
     *
     *
     * An identifier used to map this custom tool call to a tool call output.
     */
    call_id: unknown;
    /**
     * The pending safety checks for the computer call.
     */
    pending_safety_checks: Array<ComputerToolCallSafetyCheck> | null;
    acknowledged_safety_checks: Array<ComputerCallSafetyCheckParam> | null;
    /**
     * A JSON string of the output of the local shell tool call.
     *
     *
     * The output from the custom tool call generated by your code.
     */
    output: string | null;
    /**
     * A JSON string of the arguments to pass to the function.
     *
     *
     * A JSON string of arguments for the tool.
     *
     *
     * A JSON string of the arguments passed to the tool.
     */
    arguments: string | null;
    /**
     * The name of the function to run.
     *
     *
     * The name of the tool to run.
     *
     *
     * The name of the tool that was run.
     *
     *
     * The name of the custom tool being called.
     */
    name: string | null;
    encrypted_content: string | null;
    /**
     * Reasoning summary content.
     */
    summary: Array<SummaryText> | null;
    result: string | null;
    code: string | null;
    /**
     * The ID of the container used to run the code.
     */
    container_id: string | null;
    outputs: Array<CodeInterpreterOutput> | null;
    error: string | null;
    /**
     * The label of the MCP server.
     *
     *
     * The label of the MCP server making the request.
     *
     *
     * The label of the MCP server running the tool.
     */
    server_label: string | null;
    /**
     * The tools available on the server.
     */
    tools: Array<McpListToolsTool> | null;
    /**
     * The ID of the approval request being answered.
     */
    approval_request_id: string | null;
    /**
     * Whether the request was approved.
     */
    approve: boolean | null;
    reason: string | null;
    request_id: unknown;
    /**
     * The input for the custom tool call generated by the model.
     */
    input: string | null;
};

type CacheControlEphemeralType = "ephemeral";

/**
 * The time-to-live for the cache control breakpoint.
 *
 * This may be one the following values:
 * - `5m`: 5 minutes
 * - `1h`: 1 hour
 *
 * Defaults to `5m`.
 */
type Ttl = "1h" | "5m";

type CacheControlEphemeral = {
    /**
     * The time-to-live for the cache control breakpoint.
     *
     * This may be one the following values:
     * - `5m`: 5 minutes
     * - `1h`: 1 hour
     *
     * Defaults to `5m`.
     */
    ttl: Ttl | null;
    type: CacheControlEphemeralType;
};

type RequestCitationsConfig = {
    enabled: boolean | null;
};

type CitationType = "char_location" | "content_block_location" | "page_location" | "search_result_location" | "web_search_result_location";

type RequestLocationCitation = {
    cited_text: string;
    document_index: bigint | null;
    document_title: string | null;
    end_char_index: bigint | null;
    start_char_index: bigint | null;
    type: CitationType;
    end_page_number: bigint | null;
    start_page_number: bigint | null;
    end_block_index: bigint | null;
    start_block_index: bigint | null;
    encrypted_index: string | null;
    title: string | null;
    url: string | null;
    search_result_index: bigint | null;
    source: string | null;
};

type Citations = RequestCitationsConfig | Array<RequestLocationCitation>;

type SystemType = "text";

/**
 * Regular text content.
 */
type RequestTextBlock = {
    /**
     * Create a cache control breakpoint at this content block.
     */
    cache_control: CacheControlEphemeral | null;
    citations: Array<RequestLocationCitation> | null;
    text: string;
    type: SystemType;
};

type FluffyMediaType = "application/pdf" | "image/gif" | "image/jpeg" | "image/png" | "image/webp" | "text/plain";

type FluffyType = "base64" | "content" | "text" | "url";

type ContentBlockSourceContentItemType = "image" | "text";

type PurpleMediaType = "image/gif" | "image/jpeg" | "image/png" | "image/webp";

type PurpleType = "base64" | "url";

type SourceSourceClass = {
    data: string | null;
    media_type: PurpleMediaType | null;
    type: PurpleType;
    url: string | null;
};

/**
 * Regular text content.
 *
 * Image content specified directly as base64 data or as a reference via a URL.
 */
type ContentBlockSourceContentItem = {
    /**
     * Create a cache control breakpoint at this content block.
     */
    cache_control: CacheControlEphemeral | null;
    citations: Array<RequestLocationCitation> | null;
    text: string | null;
    type: ContentBlockSourceContentItemType;
    source: SourceSourceClass | null;
};

type SourceContent = Array<ContentBlockSourceContentItem> | string;

type SourceSource = {
    data: string | null;
    media_type: FluffyMediaType | null;
    type: FluffyType;
    url: string | null;
    content: SourceContent | null;
};

type Source = SourceSource | string;

type WebSearchToolResultBlockItemType = "document" | "image" | "search_result" | "text" | "web_search_result";

/**
 * Regular text content.
 *
 * Image content specified directly as base64 data or as a reference via a URL.
 *
 * A search result block containing source, title, and content from search operations.
 *
 * Document content, either specified directly as base64 data, as text, or as a reference
 * via a URL.
 */
type Block = {
    /**
     * Create a cache control breakpoint at this content block.
     */
    cache_control: CacheControlEphemeral | null;
    citations: Citations | null;
    text: string | null;
    type: WebSearchToolResultBlockItemType;
    source: Source | null;
    content: Array<RequestTextBlock> | null;
    title: string | null;
    context: string | null;
    encrypted_content: string | null;
    page_age: string | null;
    url: string | null;
};

type RequestWebSearchToolResultErrorType = "web_search_tool_result_error";

type WebSearchToolResultErrorCode = "invalid_tool_input" | "max_uses_exceeded" | "query_too_long" | "request_too_large" | "too_many_requests" | "unavailable";

type RequestWebSearchToolResultError = {
    error_code: WebSearchToolResultErrorCode;
    type: RequestWebSearchToolResultErrorType;
};

type Content = Array<Block> | RequestWebSearchToolResultError | string;

type InputContentBlockType = "document" | "image" | "redacted_thinking" | "search_result" | "server_tool_use" | "text" | "thinking" | "tool_result" | "tool_use" | "web_search_tool_result";

/**
 * Regular text content.
 *
 * Image content specified directly as base64 data or as a reference via a URL.
 *
 * Document content, either specified directly as base64 data, as text, or as a reference
 * via a URL.
 *
 * A search result block containing source, title, and content from search operations.
 *
 * A block specifying internal thinking by the model.
 *
 * A block specifying internal, redacted thinking by the model.
 *
 * A block indicating a tool use by the model.
 *
 * A block specifying the results of a tool use by the model.
 */
type InputContentBlock = {
    /**
     * Create a cache control breakpoint at this content block.
     */
    cache_control: CacheControlEphemeral | null;
    citations: Citations | null;
    text: string | null;
    type: InputContentBlockType;
    source: Source | null;
    context: string | null;
    title: string | null;
    content: Content | null;
    signature: string | null;
    thinking: string | null;
    data: string | null;
    id: string | null;
    input: unknown;
    name: string | null;
    is_error: boolean | null;
    tool_use_id: string | null;
};

type MessageContent = Array<InputContentBlock> | string;

type MessageRole = "assistant" | "user";

type InputMessage = {
    content: MessageContent;
    role: MessageRole;
};

declare global {
    type Buffer = Uint8Array;
}

type ValidationResult<T> = {
    ok: true;
    data: T;
} | {
    ok: false;
    error: {
        message: string;
    };
};
type TransformStreamChunkResult = {
    passThrough: true;
    data: unknown;
} | {
    transformed: true;
    data: unknown;
    sourceFormat: string;
};
interface StreamSessionChunk {
    data: unknown;
    eventType?: string;
}
interface TransformStreamSessionHandle {
    push(input: string): StreamSessionChunk[];
    finish(): StreamSessionChunk[];
    pushSse(input: string): string[];
    finishSse(): string[];
}

export type { AssistantContent, AssistantContentPart, BuiltinToolProvider, ChatCompletionRequestMessage, GeneratedFileContentPart, InputItem, InputMessage, JsonSchemaConfig, Message, ProviderFormat, ProviderMetadata, ProviderOptions, ReasoningCanonical, ReasoningConfig, ReasoningEffort, ResponseFormatConfig, ResponseFormatType, SourceContentPart, SourceType$1 as SourceType, StreamSessionChunk, SummaryMode, TextContentPart, TokenBudget, ToolCallArguments, ToolCallContentPart, ToolChoiceConfig, ToolChoiceMode, ToolContentPart, ToolErrorContentPart, ToolResultContentPart, ToolResultResponsePart, TransformStreamChunkResult, TransformStreamSessionHandle, UniversalParams, UniversalRequest, UniversalTool, UniversalToolType, UserContent, UserContentPart, ValidationResult };
