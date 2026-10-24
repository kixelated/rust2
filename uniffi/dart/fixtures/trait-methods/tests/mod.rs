use anyhow::Result;

#[test]
fn trait_methods() -> Result<()> {
    uniffi_dart::testing::run_test("trait_methods", "src/api.udl", None)
}
