#![deny(unsafe_code)]

use sim::sim_lib;

#[sim_lib(id = "native-fixture", version = "0.1.0", native_export = true)]
mod native_fixture {
    #[allow(unused_imports)]
    use sim::{
        case,
        kernel::{Expr, Symbol},
        sim_codec, sim_fn,
    };

    #[sim_codec(
        symbol = "codec/native-fixture",
        decode = "decode_native_fixture",
        encode = "encode_native_fixture"
    )]
    pub fn native_fixture_codec() {}

    pub fn decode_native_fixture(text: String) -> Expr {
        Expr::List(vec![
            Expr::Symbol(Symbol::qualified("native", "decoded")),
            Expr::String(text),
        ])
    }

    pub fn encode_native_fixture(expr: Expr) -> String {
        match expr {
            Expr::Symbol(symbol) => format!("encoded:{symbol}"),
            other => format!("encoded:{other:?}"),
        }
    }

    #[sim_fn(name = "native-hello")]
    #[case(args = "()", result = "String")]
    pub fn native_hello() -> String {
        "hello from native".to_owned()
    }

    #[sim_fn(name = "native-describe")]
    #[case(args = "((capture value Any))", result = "String")]
    pub fn native_describe(value: Expr) -> String {
        format!("native:{value:?}")
    }
}
