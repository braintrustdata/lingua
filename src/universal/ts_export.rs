#[cfg(test)]
mod tests {
    use super::super::{citation::*, message::*, provider::*};

    #[test]
    fn export_typescript_types() {
        // This test triggers ts-rs to export all types marked with #[ts(export)]
        // ts-rs automatically generates .ts files when running `cargo test`

        // Reference all the types to ensure they get exported
        let _: Option<Message> = None;
        let _: Option<Citation> = None;
        let _: Option<ProviderMessagePartConfig> = None;

        println!("✅ TypeScript types exported automatically to bindings/typescript/");
    }
}
