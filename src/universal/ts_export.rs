#[cfg(test)]
mod tests {
    use crate::universal::*;

    #[test]
    fn export_typescript_types() {
        // This test triggers ts-rs to export all types marked with #[ts(export)]
        // ts-rs automatically generates .ts files when running `cargo test`

        // Reference all the types to ensure they get exported
        let _: Option<LanguageModelV2Message> = None;
        let _: Option<LanguageModelV2Content> = None;
        let _: Option<LanguageModelV2SourceType> = None;
        let _: Option<SharedV2ProviderOptions> = None;
        let _: Option<SharedV2ProviderMetadata> = None;

        println!("✅ TypeScript types exported automatically to bindings/typescript/");
    }
}
