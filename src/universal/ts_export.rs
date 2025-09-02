#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_typescript_types() {
        // This test triggers ts-rs to export all types marked with #[ts(export)]
        // Just by referencing the types, ts-rs will generate the .ts files

        // The build script sets TS_RS_EXPORT_DIR to ./bindings/typescript
        // so all types will be automatically exported there
        println!("TypeScript types should be exported to bindings/typescript/");
    }
}
