struct Foo {
    this: u32,
    errno: u32,
    assert: u32,
}

fn assert_fields_are_42(value: Foo) {
    assert_eq!(42, value.this);
    assert_eq!(42, value.errno);
    assert_eq!(42, value.assert);
}

uniffi::include_scaffolding!("reserved_field_name");
