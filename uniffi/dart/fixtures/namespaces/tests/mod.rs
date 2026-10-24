use anyhow::Result;

#[test]
fn namespaces() -> Result<()> {
    uniffi_dart::testing::run_library_test("dart_namespaces")
}
