use elmir::universal::{citation::*, message::*, provider::*};
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating TypeScript bindings for Universal message format...");

    // Create bindings directory structure mirroring src/
    std::fs::create_dir_all("bindings/typescript/universal")?;

    // Generate TypeScript bindings organized by module
    println!("Exporting universal/citation types...");
    Citation::export_to("bindings/typescript/universal/CitationType.ts")?;
    CitationPosition::export_to("bindings/typescript/universal/CitationPositionType.ts")?;
    CitationSource::export_to("bindings/typescript/universal/CitationSourceType.ts")?;
    SourceType::export_to("bindings/typescript/universal/SourceTypeEnum.ts")?;

    println!("Exporting universal/provider types...");
    ProviderMessagePartConfig::export_to(
        "bindings/typescript/universal/ProviderMessagePartConfigType.ts",
    )?;
    AnthropicConfig::export_to("bindings/typescript/universal/AnthropicConfigType.ts")?;
    OpenAIConfig::export_to("bindings/typescript/universal/OpenAIConfigType.ts")?;
    GoogleConfig::export_to("bindings/typescript/universal/GoogleConfigType.ts")?;
    BedrockConfig::export_to("bindings/typescript/universal/BedrockConfigType.ts")?;
    ReasoningEffort::export_to("bindings/typescript/universal/ReasoningEffortType.ts")?;
    CacheControlEphemeral::export_to("bindings/typescript/universal/CacheControlEphemeralType.ts")?;
    CacheTtl::export_to("bindings/typescript/universal/CacheTtlType.ts")?;

    println!("Exporting universal/message types...");
    Message::export_to("bindings/typescript/universal/MessageType.ts")?;
    UserContentPart::export_to("bindings/typescript/universal/UserContentPartType.ts")?;
    AssistantContentPart::export_to("bindings/typescript/universal/AssistantContentPartType.ts")?;
    ToolContentPart::export_to("bindings/typescript/universal/ToolContentPartType.ts")?;
    ImageDetail::export_to("bindings/typescript/universal/ImageDetailType.ts")?;
    ToolResultContent::export_to("bindings/typescript/universal/ToolResultContentType.ts")?;
    SearchResultItem::export_to("bindings/typescript/universal/SearchResultItemType.ts")?;
    WebSearchResultItem::export_to("bindings/typescript/universal/WebSearchResultItemType.ts")?;
    FileData::export_to("bindings/typescript/universal/FileDataType.ts")?;
    Base64Data::export_to("bindings/typescript/universal/Base64DataType.ts")?;

    // Create index.ts files for each module (like mod.rs)
    println!("Creating index files...");
    create_citation_index()?;
    create_provider_index()?;
    create_message_index()?;
    create_universal_index()?;

    println!("✅ TypeScript bindings generated successfully!");
    println!("📁 Structure:");
    println!("  bindings/typescript/");
    println!("  └── universal/");
    println!("      ├── index.ts            # Re-exports all universal types");
    println!("      ├── citation.ts         # Citation module types");
    println!("      ├── provider.ts         # Provider config types");
    println!("      ├── message.ts          # Message types");
    println!("      └── [Type]Type.ts       # Individual generated types");

    Ok(())
}

// Create citation module index (like citation/mod.rs)
fn create_citation_index() -> Result<(), Box<dyn std::error::Error>> {
    let content = r#"// Citation module types - mirrors src/universal/citation.rs
export type { Citation } from './CitationType';
export type { CitationPosition } from './CitationPositionType';
export type { CitationSource } from './CitationSourceType';
export type { SourceType } from './SourceTypeEnum';
"#;
    std::fs::write("bindings/typescript/universal/citation.ts", content)?;
    Ok(())
}

// Create provider module index (like provider/mod.rs)
fn create_provider_index() -> Result<(), Box<dyn std::error::Error>> {
    let content = r#"// Provider module types - mirrors src/universal/provider.rs
export type { ProviderMessagePartConfig } from './ProviderMessagePartConfigType';
export type { AnthropicConfig } from './AnthropicConfigType';
export type { OpenAIConfig } from './OpenAIConfigType';
export type { GoogleConfig } from './GoogleConfigType';
export type { BedrockConfig } from './BedrockConfigType';
export type { ReasoningEffort } from './ReasoningEffortType';
export type { CacheControlEphemeral } from './CacheControlEphemeralType';
export type { CacheTtl } from './CacheTtlType';
"#;
    std::fs::write("bindings/typescript/universal/provider.ts", content)?;
    Ok(())
}

// Create message module index (like message/mod.rs)
fn create_message_index() -> Result<(), Box<dyn std::error::Error>> {
    let content = r#"// Message module types - mirrors src/universal/message.rs
export type { Message } from './MessageType';
export type { UserContentPart } from './UserContentPartType';
export type { AssistantContentPart } from './AssistantContentPartType';
export type { ToolContentPart } from './ToolContentPartType';
export type { ImageDetail } from './ImageDetailType';
export type { ToolResultContent } from './ToolResultContentType';
export type { SearchResultItem } from './SearchResultItemType';
export type { WebSearchResultItem } from './WebSearchResultItemType';
export type { FileData } from './FileDataType';
export type { Base64Data } from './Base64DataType';
"#;
    std::fs::write("bindings/typescript/universal/message.ts", content)?;
    Ok(())
}

// Create main universal index (like universal/mod.rs)
fn create_universal_index() -> Result<(), Box<dyn std::error::Error>> {
    let content = r#"// Universal module types - mirrors src/universal/mod.rs
export * from './citation';
export * from './provider';
export * from './message';

// Example usage
import type { Message, ProviderMessagePartConfig } from './message';
import type { Citation } from './citation';

export const createExampleMessage = (): Message => ({
  role: "user",
  content: [
    {
      type: "text",
      text: "Analyze this document with citations",
      citations: [
        {
          cited_text: "important finding",
          position: {
            type: "char_range",
            start: 100,
            end: 200
          },
          source: {
            url: "https://example.com",
            title: "Research Paper",
            document_title: null,
            document_index: null,
            license: null,
            source_type: "document"
          }
        }
      ],
      provider_config: {
        anthropic: {
          cache_control: {
            ttl: "1h"
          },
          extra: null
        },
        openai: {
          reasoning_effort: "high",
          audio_voice: null,
          extra: null
        },
        google: null,
        bedrock: null,
        other: null
      }
    }
  ],
  provider_config: null
});
"#;
    std::fs::write("bindings/typescript/universal/index.ts", content)?;
    Ok(())
}
