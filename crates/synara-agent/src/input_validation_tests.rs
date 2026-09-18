use super::*;
use crate::validate_input;
use std::collections::BTreeMap;
use synara_core::InputField;
use synara_core::InputValue;

fn request(kind: InputFieldKind) -> UserInputRequest {
    UserInputRequest {
        id: "question".into(),
        message: "Choose".into(),
        url: None,
        fields: vec![InputField {
            id: "value".into(),
            label: "Value".into(),
            required: true,
            kind,
        }],
    }
}
fn choices() -> Vec<SelectChoice> {
    vec![
        SelectChoice {
            value: "æ🦀".into(),
            label: "Danish".into(),
            group: None,
        },
        SelectChoice {
            value: "日本語".into(),
            label: "Japanese".into(),
            group: None,
        },
    ]
}
#[test]
fn navigation_accepts_explicit_web_destinations_but_not_ambiguous_authorities() {
    for url in [
        "http://127.0.0.1:9876/login",
        "https://example.com/日本語?state=a%20b#go",
        "http://[::1]:1234/",
        "https://xn--bcher-kva.example/",
    ] {
        assert!(validate_web_url(url).is_ok(), "{url}");
    }
    for url in [
        "file:///tmp/x",
        "javascript:alert(1)",
        "https://",
        "https://example.com:65536",
        "https://example.com:",
        "https://[::1",
        "https://[no-ip]/",
        "https://example.com\\@evil.com",
        "https://user:secret@example.com",
        "https://%65xample.com",
        "https://x..example",
        "https://-x.example",
        "https://example.com/%xx",
        "https://example.com/\nnext",
        "https://example.com:80:90",
        "https://example.com/#\u{0000}",
    ] {
        assert!(validate_web_url(url).is_err(), "{url:?}");
    }
}
#[test]
fn impossible_schemas_and_duplicate_controls_never_reach_the_ui() {
    for kind in [
        InputFieldKind::Text {
            min_length: Some(10),
            max_length: Some(1),
            format: None,
        },
        InputFieldKind::Text {
            min_length: None,
            max_length: None,
            format: Some("custom".into()),
        },
        InputFieldKind::Number {
            integer: false,
            minimum: Some(f64::NAN),
            maximum: None,
        },
        InputFieldKind::Number {
            integer: true,
            minimum: Some(0.1),
            maximum: Some(0.9),
        },
        InputFieldKind::MultiChoice {
            options: choices(),
            minimum: Some(3),
            maximum: None,
        },
        InputFieldKind::Choice { options: vec![] },
    ] {
        assert!(validate_input_request(&request(kind)).is_err());
    }
    let mut schema = request(InputFieldKind::Boolean);
    schema.fields.push(schema.fields[0].clone());
    assert!(validate_input_request(&schema).is_err());
    schema.fields.pop();
    schema.url = Some("https://example.com".into());
    assert!(validate_input_request(&schema).is_err());
}
#[test]
fn unicode_single_and_multiple_choices_reject_unknown_repeated_and_missing_values() {
    let schema = request(InputFieldKind::MultiChoice {
        options: choices(),
        minimum: Some(1),
        maximum: Some(2),
    });
    assert!(
        validate_input(
            &schema,
            &BTreeMap::from([("value".into(), InputValue::Strings(vec!["日本語".into()]))])
        )
        .is_ok()
    );
    for values in [vec![], vec!["unknown"], vec!["æ🦀", "æ🦀"]] {
        assert!(
            validate_input(
                &schema,
                &BTreeMap::from([(
                    "value".into(),
                    InputValue::Strings(values.into_iter().map(str::to_owned).collect())
                )])
            )
            .is_err()
        );
    }
    assert!(validate_input(&schema, &BTreeMap::new()).is_err());
    let schema = request(InputFieldKind::Choice { options: choices() });
    assert!(
        validate_input(
            &schema,
            &BTreeMap::from([("value".into(), InputValue::Text("æ🦀".into()))])
        )
        .is_ok()
    );
}
#[test]
fn formats_are_enforced_and_invalid_unicode_never_panics() {
    for (format, valid, invalid) in [
        ("date", "2024-02-29", "2025-02-29"),
        (
            "date-time",
            "2026-09-17T18:00:01.123+02:00",
            "2026-09-17T24:00:00Z",
        ),
        ("email", "name+tag@example.com", "name..tag@example.com"),
        ("uri", "urn:example:abc", "not a URI"),
    ] {
        let schema = request(InputFieldKind::Text {
            min_length: None,
            max_length: None,
            format: Some(format.into()),
        });
        assert!(
            validate_input(
                &schema,
                &BTreeMap::from([("value".into(), InputValue::Text(valid.into()))])
            )
            .is_ok()
        );
        for text in [
            invalid,
            "日本語🦀",
            "",
            "2026-+1-01",
            "2026-01-01T+1:00:00Z",
        ] {
            assert!(!valid_text_format(text, Some(format)), "{format}: {text}");
        }
    }
    for text in ["2026-09-17T01:02:03Z", "2026-09-17t01:02:03z"] {
        assert!(valid_text_format(text, Some("date-time")));
    }
}
#[test]
fn aggregate_presentation_budget_is_bounded() {
    let mut schema = request(InputFieldKind::Choice {
        options: (0..512)
            .map(|i| SelectChoice {
                value: i.to_string(),
                label: "x".repeat(1024),
                group: None,
            })
            .collect(),
    });
    assert!(matches!(
        validate_input_request(&schema),
        Err(AgentError::Limit)
    ));
    schema.fields[0].kind = InputFieldKind::Text {
        min_length: Some(2),
        max_length: Some(2),
        format: None,
    };
    assert!(
        validate_input(
            &schema,
            &BTreeMap::from([("value".into(), InputValue::Text("æ🦀".into()))])
        )
        .is_ok()
    );
}
