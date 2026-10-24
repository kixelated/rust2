use anyhow::Result;

#[test]
fn trait_interfaces() -> Result<()> {
    uniffi_dart::testing::run_test("trait_interfaces", "src/api.udl", None)
}
