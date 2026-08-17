use fack_codegen::Target;

fn expand(source: &str) -> String {
    let input = syn::parse_str(source).expect("test derive input must parse");
    Target::input(&input)
        .and_then(Target::validate)
        .and_then(fack_codegen::ValidatedTarget::expand)
        .expect("test derive input must validate and expand")
        .to_string()
}

#[test]
fn static_messages_use_formatter_write_str() {
    let output = expand(
        r#"
        #[error("static message")]
        struct Static;
        "#,
    );

    assert!(output.contains("write_str"));
    assert!(!output.contains("write !"));
}

#[test]
fn source_less_errors_do_not_override_source() {
    let output = expand(
        r#"
        #[error("static message")]
        struct Static;
        "#,
    );

    assert!(!output.contains("fn source"));
}

#[test]
fn ordinary_and_transparent_sources_expand_differently() {
    let ordinary = expand(
        r#"
        #[error("outer")]
        #[error(source(inner))]
        struct Outer { inner: std::io::Error }
        "#,
    );
    let transparent = expand(
        r#"
        #[error(transparent(inner))]
        struct Transparent { inner: std::io::Error }
        "#,
    );

    assert!(ordinary.contains("Some"));
    assert!(transparent.contains("Error :: source"));
}
