/// Creates a primitive::Node referencing something in the lang_env.
#[macro_export]
macro_rules! amlang_node {
    ($local:ident, $context:expr) => {{
        let ctx = $context;
        $crate::primitive::Node::new(ctx.lang_env(), ctx.$local())
    }};
}
