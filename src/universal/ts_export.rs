#[cfg(test)]
mod tests {
    use crate::universal::*;

    #[test]
    fn export_typescript_types() {
        // This test triggers ts-rs to export all types marked with #[ts(export)]
        // ts-rs automatically generates .ts files when running `cargo test`

        // Reference all the NEW types to ensure they get exported
        let _: Option<ModelMessage> = None;
        let _: Option<ModelPrompt> = None;
        let _: Option<UserContent> = None;
        let _: Option<AssistantContent> = None;
        let _: Option<UserContentPart> = None;
        let _: Option<AssistantContentPart> = None;
        let _: Option<TextPart> = None;
        let _: Option<ImagePart> = None;
        let _: Option<FilePart> = None;
        let _: Option<ReasoningPart> = None;
        let _: Option<ToolCallPart> = None;
        let _: Option<ToolResultPart> = None;
        let _: Option<SourceType> = None;
        let _: Option<ProviderOptions> = None;
        let _: Option<ProviderMetadata> = None;

        // Legacy types for backward compatibility
        let _: Option<LanguageModelV2Message> = None;
        let _: Option<LanguageModelV2Content> = None;
        let _: Option<LanguageModelV2UserContent> = None;
        let _: Option<LanguageModelV2AssistantContent> = None;
        let _: Option<LanguageModelV2ToolContent> = None;
        let _: Option<LanguageModelV2SourceType> = None;
        let _: Option<SharedV2ProviderOptions> = None;
        let _: Option<SharedV2ProviderMetadata> = None;

        println!("✅ TypeScript types exported automatically to bindings/typescript/");
    }
}
