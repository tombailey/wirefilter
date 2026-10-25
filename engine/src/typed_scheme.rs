use crate::{IdentifierRegistrationError, Scheme, SchemeBuilder};

// Public because exported macro expansions must access these from other crates.
#[doc(hidden)]
pub trait TypedSchemeDefinition {
    type Fields<'s>;

    fn register(
        builder: &mut SchemeBuilder,
        prefix: &str,
    ) -> Result<(), IdentifierRegistrationError>;
    fn fields<'s>(scheme: &'s Scheme, prefix: &str) -> Self::Fields<'s>;
}

#[doc(hidden)]
pub fn typed_field_name(prefix: &str, member: &str) -> String {
    let member = member.strip_prefix("r#").unwrap_or(member);
    format!("{prefix}{member}")
}

/// Declares a named scheme wrapper and its borrowed field view.
///
/// Supply both names explicitly and at least one field. Scalar and
/// container types use the same syntax as [`crate::Scheme!`]. `optional(Type)`
/// declares an optional field; `namespace(Schema)` embeds another declaration's
/// fields under a prefix. Only field declarations are composed, not functions,
/// lists, or configuration from an existing scheme instance.
///
/// ```
/// use wirefilter::{TypedScheme, ExecutionContext};
/// TypedScheme! {
///     pub struct Http => HttpFields {
///         method: Bytes,
///         ua: optional(Bytes),
///     }
/// }
/// TypedScheme! {
///     pub struct Request => RequestFields {
///         http: namespace(Http),
///         port: Int,
///         tags: Array(Bytes),
///     }
/// }
/// let scheme = Request::new();
/// let fields = scheme.get_fields();
/// let mut ctx = ExecutionContext::new(&scheme);
/// ctx.set_field_value(fields.http.method, "GET")?;
/// ctx.set_field_value(fields.port, 443)?;
/// let filter = scheme.parse("http.method == \"GET\" && port == 443")?.compile();
/// assert!(filter.execute(&ctx)?);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Undeclared members are rejected by Rust:
///
/// ```compile_fail
/// use wirefilter::TypedScheme;
/// TypedScheme! { struct Http => HttpFields { method: Bytes } }
/// let scheme = Http::new();
/// let _ = scheme.get_fields().methd;
/// ```
///
/// Views cannot outlive their scheme:
///
/// ```compile_fail
/// use wirefilter::TypedScheme;
/// TypedScheme! { struct Http => HttpFields { method: Bytes } }
/// let fields = { let scheme = Http::new(); scheme.get_fields() };
/// let _ = fields.method;
/// ```
///
/// Names must be valid Wirefilter identifier segments. Raw Rust identifiers
/// such as `r#type` register the unescaped name `type`. `new()` panics for names
/// rejected by Wirefilter (for example, Unicode identifiers).
///
/// The wrapper dereferences to an immutable [`crate::Scheme`], so existing
/// parsing and execution APIs remain available. Cloning preserves scheme
/// identity; constructing another instance creates a distinct scheme.
/// `get_fields()` resolves names internally on each call; retain the view when
/// populating multiple contexts. Members are ordinary [`crate::FieldRef`]
/// handles: assigned value types are still checked at runtime.
#[macro_export]
macro_rules! TypedScheme {
    (
        $vis:vis struct $name:ident => $fields:ident {
            $($field:ident: $kind:ident $(($($args:tt)*))?),+ $(,)?
        }
    ) => {
        #[derive(Clone, Debug)]
        $vis struct $name {
            scheme: $crate::Scheme,
        }

        #[derive(Clone, Copy, Debug)]
        $vis struct $fields<'s> {
            $(pub $field: $crate::TypedScheme!(@type $kind $(($($args)*))?),)*
        }

        impl $name {
            pub fn new() -> Self {
                let mut builder = $crate::SchemeBuilder::new();
                <Self as $crate::TypedSchemeDefinition>::register(&mut builder, "")
                    .expect("invalid typed scheme declaration");
                Self { scheme: builder.build() }
            }

            pub fn get_fields(&self) -> $fields<'_> {
                <Self as $crate::TypedSchemeDefinition>::fields(&self.scheme, "")
            }
        }

        impl ::std::default::Default for $name {
            fn default() -> Self { Self::new() }
        }

        impl ::std::ops::Deref for $name {
            type Target = $crate::Scheme;
            fn deref(&self) -> &Self::Target { &self.scheme }
        }

        impl $crate::TypedSchemeDefinition for $name {
            type Fields<'s> = $fields<'s>;

            fn register(
                builder: &mut $crate::SchemeBuilder,
                prefix: &str,
            ) -> ::std::result::Result<(), $crate::IdentifierRegistrationError> {
                $(
                    let name = $crate::typed_field_name(prefix, stringify!($field));
                    $crate::TypedScheme!(@register builder, name, $kind $(($($args)*))?);
                )*
                Ok(())
            }

            fn fields<'s>(scheme: &'s $crate::Scheme, prefix: &str) -> Self::Fields<'s> {
                $fields {
                    $($field: {
                        let name = $crate::typed_field_name(prefix, stringify!($field));
                        $crate::TypedScheme!(@value scheme, name, $kind $(($($args)*))?)
                    },)*
                }
            }
        }
    };
    (@type namespace($schema:path)) => {
        <$schema as $crate::TypedSchemeDefinition>::Fields<'s>
    };
    (@type $kind:ident $(($($args:tt)*))?) => { $crate::FieldRef<'s> };
    (@register $builder:ident, $name:ident, namespace($schema:path)) => {
        <$schema as $crate::TypedSchemeDefinition>::register($builder, &format!("{}.", $name))?;
    };
    (@register $builder:ident, $name:ident, optional($($ty:tt)*)) => {
        $builder.add_optional_field($name, $crate::Scheme!($($ty)*))?;
    };
    (@register $builder:ident, $name:ident, $($ty:tt)*) => {
        $builder.add_field($name, $crate::Scheme!($($ty)*))?;
    };
    (@value $scheme:ident, $name:ident, namespace($schema:path)) => {
        <$schema as $crate::TypedSchemeDefinition>::fields($scheme, &format!("{}.", $name))
    };
    (@value $scheme:ident, $name:ident, $($ty:tt)*) => {
        $scheme.get_field(&$name).expect("registered typed scheme field")
    };
}
