use elmir::universal::{citation::*, message::*, provider::*};
use std::fs;
use ts_rs::TS;

fn main() {
    // Create output directory
    fs::create_dir_all("bindings/typescript").unwrap();

    // The simplest approach: just call export() on each type
    // ts-rs handles the rest automatically
    Citation::export_to("bindings/typescript/Citation.ts").unwrap();
    CitationPosition::export_to("bindings/typescript/CitationPosition.ts").unwrap();
    CitationSource::export_to("bindings/typescript/CitationSource.ts").unwrap();
    SourceType::export_to("bindings/typescript/SourceType.ts").unwrap();

    Message::export_to("bindings/typescript/Message.ts").unwrap();
    UserContentPart::export_to("bindings/typescript/UserContentPart.ts").unwrap();
    AssistantContentPart::export_to("bindings/typescript/AssistantContentPart.ts").unwrap();
    ToolContentPart::export_to("bindings/typescript/ToolContentPart.ts").unwrap();

    ProviderMessagePartConfig::export_to("bindings/typescript/ProviderMessagePartConfig.ts")
        .unwrap();
    AnthropicConfig::export_to("bindings/typescript/AnthropicConfig.ts").unwrap();
    OpenAIConfig::export_to("bindings/typescript/OpenAIConfig.ts").unwrap();
    GoogleConfig::export_to("bindings/typescript/GoogleConfig.ts").unwrap();
    BedrockConfig::export_to("bindings/typescript/BedrockConfig.ts").unwrap();
    ReasoningEffort::export_to("bindings/typescript/ReasoningEffort.ts").unwrap();
    CacheControlEphemeral::export_to("bindings/typescript/CacheControlEphemeral.ts").unwrap();
    CacheTtl::export_to("bindings/typescript/CacheTtl.ts").unwrap();

    // Generate all the other supporting types
    ImageDetail::export_to("bindings/typescript/ImageDetail.ts").unwrap();
    ToolResultContent::export_to("bindings/typescript/ToolResultContent.ts").unwrap();
    SearchResultItem::export_to("bindings/typescript/SearchResultItem.ts").unwrap();
    WebSearchResultItem::export_to("bindings/typescript/WebSearchResultItem.ts").unwrap();
    FileData::export_to("bindings/typescript/FileData.ts").unwrap();
    Base64Data::export_to("bindings/typescript/Base64Data.ts").unwrap();

    // Create a simple index file
    let index_content = r#"// Universal TypeScript types - auto-generated
export * from './Citation';
export * from './CitationPosition';
export * from './CitationSource';
export * from './SourceType';
export * from './Message';
export * from './UserContentPart';
export * from './AssistantContentPart';
export * from './ToolContentPart';
export * from './ProviderMessagePartConfig';
export * from './AnthropicConfig';
export * from './OpenAIConfig';
export * from './GoogleConfig';
export * from './BedrockConfig';
export * from './ReasoningEffort';
export * from './CacheControlEphemeral';
export * from './CacheTtl';
export * from './ImageDetail';
export * from './ToolResultContent';
export * from './SearchResultItem';
export * from './WebSearchResultItem';
export * from './FileData';
export * from './Base64Data';
"#;

    fs::write("bindings/typescript/index.ts", index_content).unwrap();

    println!("✅ TypeScript types exported automatically!");
    println!("📁 Files generated in bindings/typescript/");
}
