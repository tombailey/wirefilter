use wirefilter::{ExecutionContext, GetType, SetFieldValueError, Type, TypedScheme};

mod declarations {
    use wirefilter::TypedScheme;

    TypedScheme! {
        pub struct Http => HttpFields {
            method: Bytes,
            ua: optional(Bytes),
            r#type: Int,
        }
    }
}

TypedScheme! {
    struct Request => RequestFields {
        http: namespace(declarations::Http),
        backup: namespace(declarations::Http),
        port: Int,
        tags: optional(Array(Map(Bytes))),
    }
}

TypedScheme! {
    struct Envelope => EnvelopeFields {
        request: namespace(Request),
    }
}

#[test]
fn nested_fields_execute_in_parent_scheme() {
    let scheme = Envelope::new();
    let fields = scheme.get_fields();
    assert_eq!(fields.request.http.method.name(), "request.http.method");
    assert_eq!(fields.request.backup.method.name(), "request.backup.method");
    assert_eq!(fields.request.http.r#type.name(), "request.http.type");
    assert_eq!(
        fields.request.http.method,
        scheme.get_field("request.http.method").unwrap()
    );
    assert!(!fields.request.http.method.optional());
    assert!(fields.request.http.ua.optional());
    assert!(fields.request.tags.optional());
    assert_eq!(
        fields.request.tags.get_type(),
        Type::Array(Type::Map(Type::Bytes.into()).into())
    );

    let mut ctx: ExecutionContext<'_> = ExecutionContext::new(&scheme);
    ctx.set_field_value(fields.request.http.method, "GET")
        .unwrap();
    ctx.set_field_value(fields.request.port, 443).unwrap();
    let filter = scheme
        .parse(
            "request.http.method == \"GET\" && request.port == 443 && request.http.ua != \"bot\"",
        )
        .unwrap()
        .compile();
    assert!(filter.execute(&ctx).unwrap());
}

#[test]
fn identity_and_value_checks_are_preserved() {
    let scheme = Request::new();
    let cloned = scheme.clone();
    let other = Request::default();
    let standalone = declarations::Http::new();
    let mut ctx: ExecutionContext<'_> = ExecutionContext::new(&scheme);
    ctx.set_field_value(cloned.get_fields().http.method, "GET")
        .unwrap();
    assert!(matches!(
        ctx.set_field_value(other.get_fields().http.method, "GET"),
        Err(SetFieldValueError::SchemeMismatch(_))
    ));
    assert!(matches!(
        ctx.set_field_value(standalone.get_fields().method, "GET"),
        Err(SetFieldValueError::SchemeMismatch(_))
    ));
    assert!(
        ctx.set_field_value(scheme.get_fields().port, "443")
            .is_err()
    );
}
