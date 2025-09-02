use elmir::universal::{citation::*, message::*, provider::*};
use ts_rs::TS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating TypeScript bindings for Universal message format...");

    // Create bindings directory
    std::fs::create_dir_all("bindings/typescript")?;

    // Generate all TypeScript bindings by calling export_to on each type
    println!("Exporting Citation types...");
    Citation::export_to("bindings/typescript/Citation.ts")?;
    CitationPosition::export_to("bindings/typescript/CitationPosition.ts")?;
    CitationSource::export_to("bindings/typescript/CitationSource.ts")?;
    SourceType::export_to("bindings/typescript/SourceType.ts")?;

    println!("Exporting Provider config types...");
    ProviderMessagePartConfig::export_to("bindings/typescript/ProviderMessagePartConfig.ts")?;
    AnthropicConfig::export_to("bindings/typescript/AnthropicConfig.ts")?;
    OpenAIConfig::export_to("bindings/typescript/OpenAIConfig.ts")?;
    GoogleConfig::export_to("bindings/typescript/GoogleConfig.ts")?;
    BedrockConfig::export_to("bindings/typescript/BedrockConfig.ts")?;
    ReasoningEffort::export_to("bindings/typescript/ReasoningEffort.ts")?;
    CacheControlEphemeral::export_to("bindings/typescript/CacheControlEphemeral.ts")?;
    CacheTtl::export_to("bindings/typescript/CacheTtl.ts")?;

    println!("Exporting Message types...");
    Message::export_to("bindings/typescript/UniversalMessage.ts")?;
    UserContentPart::export_to("bindings/typescript/UserContentPart.ts")?;
    AssistantContentPart::export_to("bindings/typescript/AssistantContentPart.ts")?;
    ToolContentPart::export_to("bindings/typescript/ToolContentPart.ts")?;

    println!("Exporting Supporting types...");
    ImageDetail::export_to("bindings/typescript/ImageDetail.ts")?;
    ToolResultContent::export_to("bindings/typescript/ToolResultContent.ts")?;
    SearchResultItem::export_to("bindings/typescript/SearchResultItem.ts")?;
    WebSearchResultItem::export_to("bindings/typescript/WebSearchResultItem.ts")?;
    FileData::export_to("bindings/typescript/FileData.ts")?;
    Base64Data::export_to("bindings/typescript/Base64Data.ts")?;

    println!("✅ TypeScript bindings generated successfully in bindings/typescript/");
    println!("🔍 Check the following files:");

    // List generated files
    let paths = std::fs::read_dir("bindings/typescript/")?;
    let mut files: Vec<String> = paths
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "ts" {
                Some(path.file_name()?.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();

    files.sort();
    for file in files {
        if file.starts_with("Citation")
            || file.starts_with("Provider")
            || file.starts_with("Message")
            || file.starts_with("User")
            || file.starts_with("Assistant")
            || file.starts_with("Tool")
            || file.starts_with("Anthropic")
            || file.starts_with("OpenAI")
            || file.starts_with("Google")
            || file.starts_with("Bedrock")
            || file.starts_with("Reasoning")
            || file.starts_with("Cache")
        {
            println!("  📄 {}", file);
        }
    }

    Ok(())
}
