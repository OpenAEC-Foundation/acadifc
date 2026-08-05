#![cfg(feature = "ifccad")]

use acadrust::ifccad::canonicalization::{canonicalize, CanonicalValue};

#[test]
fn canonicalizes_supported_scalars_to_exact_utf8() {
    let cases = [
        (CanonicalValue::Null, r#"{"type":"null"}"#),
        (
            CanonicalValue::Bool(false),
            r#"{"type":"bool","value":false}"#,
        ),
        (
            CanonicalValue::Integer("18446744073709551615".to_owned()),
            r#"{"type":"int","value":"18446744073709551615"}"#,
        ),
        (
            CanonicalValue::Float("-0x0.0p+0".to_owned()),
            r#"{"type":"float","value":"-0x0.0p+0"}"#,
        ),
        (
            CanonicalValue::String("café".to_owned()),
            r#"{"type":"string","value":"café"}"#,
        ),
    ];

    for (value, expected) in cases {
        assert_eq!(
            canonicalize(&value).expect("canonical value"),
            expected.as_bytes()
        );
    }
}

#[test]
fn rejects_noncanonical_numeric_text_with_stable_codes() {
    let cases = [
        (
            CanonicalValue::Integer("01".to_owned()),
            "VECTOR_INTEGER_INVALID",
        ),
        (
            CanonicalValue::Integer("-0".to_owned()),
            "VECTOR_INTEGER_INVALID",
        ),
        (
            CanonicalValue::Float("1.0".to_owned()),
            "VECTOR_FLOAT_INVALID",
        ),
        (
            CanonicalValue::Float("nan".to_owned()),
            "VECTOR_FLOAT_INVALID",
        ),
    ];

    for (value, expected_code) in cases {
        let error = canonicalize(&value).expect_err("invalid canonical value");
        assert_eq!(error.code().as_str(), expected_code);
    }
}
