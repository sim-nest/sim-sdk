use sim_kernel::{
    CapabilityName, Cx, Expr, Object, ObjectCompat, Result, Symbol, Test, TestReport, Value,
};
#[cfg(any(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64",
    feature = "codec-chat",
    feature = "codec-algol"
))]
use sim_kernel::{EncodeOptions, ReadPolicy};

use super::test_runs;

/// Expectation a [`SimTest`] checks against the evaluated expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestExpected {
    /// The result must be truthy.
    Truthy,
    /// The result must equal this expression.
    Value(Expr),
    /// Evaluation must fail with an error containing this substring.
    ErrorContains(String),
    /// The expression must round-trip unchanged through these codecs.
    RoundTrip {
        /// Codecs applied in sequence for the round-trip check.
        codecs: Vec<Symbol>,
    },
}

impl TestExpected {
    /// Returns the symbol naming this expectation mode.
    pub fn mode(&self) -> Symbol {
        match self {
            Self::Truthy => Symbol::qualified("test", "truthy"),
            Self::Value(_) => Symbol::qualified("test", "value"),
            Self::ErrorContains(_) => Symbol::qualified("test", "error-contains"),
            Self::RoundTrip { .. } => Symbol::qualified("test", "round-trip"),
        }
    }

    fn expected_codec(&self, expr_codec: &Symbol) -> Option<Symbol> {
        matches!(self, Self::Value(_)).then(|| expr_codec.clone())
    }
}

/// Executable test or example: an expression, its expectation, and the codec
/// and capability context it runs in.
#[derive(Clone)]
pub struct SimTest {
    /// Name the test is registered under.
    pub name: Symbol,
    /// Lib the test belongs to.
    pub lib: Symbol,
    /// Expression evaluated when the test runs.
    pub expr: Expr,
    /// Expectation checked against the result.
    pub expected: TestExpected,
    /// Symbols the test exercises, for reporting and discovery.
    pub subjects: Vec<Symbol>,
    /// Codec the test expression round-trips through before evaluation.
    pub expr_codec: Symbol,
    /// Codec the expected value round-trips through, if any.
    pub expected_codec: Option<Symbol>,
    /// Whether the test also serves as a documentation example.
    pub example: bool,
    /// Capabilities the test requires to run.
    pub capabilities: Vec<CapabilityName>,
}

impl SimTest {
    /// Creates a truthiness-by-default test from its name, lib, expression,
    /// expectation, and subjects.
    pub fn new(
        name: Symbol,
        lib: Symbol,
        expr: Expr,
        expected: TestExpected,
        subjects: Vec<Symbol>,
    ) -> Self {
        let expr_codec = default_test_codec();
        let expected_codec = expected.expected_codec(&expr_codec);
        Self {
            name,
            lib,
            expr,
            expected,
            subjects,
            expr_codec,
            expected_codec,
            example: false,
            capabilities: Vec::new(),
        }
    }

    /// Sets the expression codec and returns the updated test.
    pub fn with_expr_codec(mut self, codec: Symbol) -> Self {
        self.expr_codec = codec;
        self
    }

    /// Sets the expected-value codec and returns the updated test.
    pub fn with_expected_codec(mut self, codec: Symbol) -> Self {
        self.expected_codec = Some(codec);
        self
    }

    /// Marks the test as a documentation example and returns it.
    pub fn as_example(mut self) -> Self {
        self.example = true;
        self
    }

    /// Adds a required capability and returns the updated test.
    pub fn requiring(mut self, capability: CapabilityName) -> Self {
        self.capabilities.push(capability);
        self
    }

    fn run_inner(&self, cx: &mut Cx) -> Result<TestReport> {
        let expr = match checked_roundtrip(cx, &self.expr, &self.expr_codec, "expr")? {
            Ok(expr) => expr,
            Err(detail) => return Ok(failed_report(self.name.clone(), detail)),
        };
        match &self.expected {
            TestExpected::Truthy => {
                let value = cx.eval_expr(expr)?;
                let passed = value.object().truth(cx)?;
                Ok(TestReport::from_result(
                    self.name.clone(),
                    passed,
                    if passed {
                        None
                    } else {
                        Some("test result was not truthy".to_owned())
                    },
                ))
            }
            TestExpected::Value(expected) => {
                let expected = match self.checked_expected(cx, expected)? {
                    Ok(expected) => expected,
                    Err(detail) => return Ok(failed_report(self.name.clone(), detail)),
                };
                match cx.eval_expr(expr) {
                    Ok(value) => {
                        let actual = value.object().as_expr(cx)?;
                        Ok(TestReport::from_result(
                            self.name.clone(),
                            actual == expected,
                            (actual != expected)
                                .then(|| format!("expected {expected:?}, found {actual:?}")),
                        ))
                    }
                    Err(error) => Ok(TestReport::from_result(
                        self.name.clone(),
                        false,
                        Some(format!("evaluation failed: {error}")),
                    )),
                }
            }
            TestExpected::ErrorContains(expected) => match cx.eval_expr(expr) {
                Ok(value) => Ok(TestReport::from_result(
                    self.name.clone(),
                    false,
                    Some(format!(
                        "expected error containing {expected:?}, found value {:?}",
                        value.object().as_expr(cx)?
                    )),
                )),
                Err(error) => {
                    let message = error.to_string();
                    Ok(TestReport::from_result(
                        self.name.clone(),
                        message.contains(expected),
                        (!message.contains(expected)).then(|| {
                            format!("expected error containing {expected:?}, found {message}")
                        }),
                    ))
                }
            },
            TestExpected::RoundTrip { codecs } => {
                let passed = roundtrip_expr(cx, &expr, codecs)?;
                Ok(TestReport::from_result(
                    self.name.clone(),
                    passed,
                    (!passed).then(|| "round-trip value changed".to_owned()),
                ))
            }
        }
    }

    fn checked_expected(
        &self,
        cx: &mut Cx,
        expected: &Expr,
    ) -> Result<std::result::Result<Expr, String>> {
        match &self.expected_codec {
            Some(codec) => checked_roundtrip(cx, expected, codec, "expected"),
            None => Ok(Ok(expected.clone())),
        }
    }
}

impl Test for SimTest {
    fn symbol(&self) -> Symbol {
        self.name.clone()
    }

    fn lib(&self) -> Symbol {
        self.lib.clone()
    }

    fn describe(&self, cx: &mut Cx) -> Result<Value> {
        self.as_table(cx)
    }

    fn run(&self, cx: &mut Cx) -> Result<TestReport> {
        test_runs::run_effect_backed(
            cx,
            self.name.clone(),
            self.expected.mode(),
            &self.capabilities,
            |cx| self.run_inner(cx),
        )
    }
}

impl Object for SimTest {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok(format!("#<test {}>", self.name))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for SimTest {
    fn class(&self, cx: &mut Cx) -> Result<sim_kernel::ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&sim_kernel::Symbol::qualified("core", "Test"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            sim_kernel::CORE_TEST_CLASS_ID,
            sim_kernel::Symbol::qualified("core", "Test"),
        )
    }
    fn as_expr(&self, _cx: &mut Cx) -> Result<Expr> {
        Ok(Expr::Symbol(self.name.clone()))
    }
    fn as_table(&self, cx: &mut Cx) -> Result<Value> {
        let mut entries = vec![
            (Symbol::new("name"), cx.factory().symbol(self.name.clone())?),
            (
                Symbol::new("subjects"),
                cx.factory().list(
                    self.subjects
                        .iter()
                        .cloned()
                        .map(|symbol| cx.factory().symbol(symbol))
                        .collect::<Result<Vec<_>>>()?,
                )?,
            ),
            (Symbol::new("lib"), cx.factory().symbol(self.lib.clone())?),
            (
                Symbol::new("mode"),
                cx.factory().symbol(self.expected.mode())?,
            ),
            (Symbol::new("expr"), cx.factory().expr(self.expr.clone())?),
            (
                Symbol::new("expr-codec"),
                cx.factory().symbol(self.expr_codec.clone())?,
            ),
        ];
        match &self.expected {
            TestExpected::Truthy => {
                entries.push((Symbol::new("expected"), cx.factory().nil()?));
                entries.push((Symbol::new("expected-codec"), cx.factory().nil()?));
                entries.push((Symbol::new("expected-error"), cx.factory().nil()?));
                entries.push((Symbol::new("codecs"), cx.factory().list(Vec::new())?));
            }
            TestExpected::Value(expected) => {
                entries.push((
                    Symbol::new("expected"),
                    cx.factory().expr(expected.clone())?,
                ));
                entries.push((
                    Symbol::new("expected-codec"),
                    match &self.expected_codec {
                        Some(codec) => cx.factory().symbol(codec.clone())?,
                        None => cx.factory().nil()?,
                    },
                ));
                entries.push((Symbol::new("expected-error"), cx.factory().nil()?));
                entries.push((Symbol::new("codecs"), cx.factory().list(Vec::new())?));
            }
            TestExpected::ErrorContains(expected) => {
                entries.push((Symbol::new("expected"), cx.factory().nil()?));
                entries.push((Symbol::new("expected-codec"), cx.factory().nil()?));
                entries.push((
                    Symbol::new("expected-error"),
                    cx.factory().string(expected.clone())?,
                ));
                entries.push((Symbol::new("codecs"), cx.factory().list(Vec::new())?));
            }
            TestExpected::RoundTrip { codecs } => {
                entries.push((Symbol::new("expected"), cx.factory().nil()?));
                entries.push((Symbol::new("expected-codec"), cx.factory().nil()?));
                entries.push((Symbol::new("expected-error"), cx.factory().nil()?));
                entries.push((
                    Symbol::new("codecs"),
                    cx.factory().list(
                        codecs
                            .iter()
                            .cloned()
                            .map(|symbol| cx.factory().symbol(symbol))
                            .collect::<Result<Vec<_>>>()?,
                    )?,
                ));
            }
        }
        entries.push((Symbol::new("example"), cx.factory().bool(self.example)?));
        entries.push((
            Symbol::new("capabilities"),
            cx.factory().list(
                self.capabilities
                    .iter()
                    .map(|capability| cx.factory().symbol(capability.as_symbol()))
                    .collect::<Result<Vec<_>>>()?,
            )?,
        ));
        cx.factory().table(entries)
    }
}

pub(crate) fn roundtrip_expr(cx: &mut Cx, expr: &Expr, codecs: &[Symbol]) -> Result<bool> {
    #[cfg(any(
        feature = "codec-lisp",
        feature = "codec-json",
        feature = "codec-binary",
        feature = "codec-binary-base64",
        feature = "codec-chat",
        feature = "codec-algol"
    ))]
    {
        let mut current = expr.clone();
        for codec in codecs {
            current = codec_roundtrip(cx, &current, codec)?;
        }
        Ok(current == *expr)
    }
    #[cfg(not(any(
        feature = "codec-lisp",
        feature = "codec-json",
        feature = "codec-binary",
        feature = "codec-binary-base64",
        feature = "codec-chat",
        feature = "codec-algol"
    )))]
    {
        let _ = (cx, expr, codecs);
        Err(sim_kernel::Error::HostError(
            "round-trip tests require at least one codec feature".to_owned(),
        ))
    }
}

fn checked_roundtrip(
    cx: &mut Cx,
    expr: &Expr,
    codec: &Symbol,
    label: &str,
) -> Result<std::result::Result<Expr, String>> {
    let decoded = match codec_roundtrip(cx, expr, codec) {
        Ok(decoded) => decoded,
        Err(error) => return Ok(Err(format!("{label} codec {codec} failed: {error}"))),
    };
    if decoded == *expr {
        Ok(Ok(decoded))
    } else {
        Ok(Err(format!(
            "{label} codec {codec} did not preserve expression: expected {expr:?}, decoded {decoded:?}"
        )))
    }
}

fn codec_roundtrip(cx: &mut Cx, expr: &Expr, codec: &Symbol) -> Result<Expr> {
    #[cfg(any(
        feature = "codec-lisp",
        feature = "codec-json",
        feature = "codec-binary",
        feature = "codec-binary-base64",
        feature = "codec-chat",
        feature = "codec-algol"
    ))]
    {
        let output = sim_codec::encode_with_codec(cx, codec, expr, EncodeOptions::default())?;
        let input = match output {
            sim_codec::Output::Text(text) => sim_codec::Input::Text(text),
            sim_codec::Output::Bytes(bytes) => sim_codec::Input::Bytes(bytes),
        };
        sim_codec::decode_with_codec(cx, codec, input, ReadPolicy::default())
    }
    #[cfg(not(any(
        feature = "codec-lisp",
        feature = "codec-json",
        feature = "codec-binary",
        feature = "codec-binary-base64",
        feature = "codec-chat",
        feature = "codec-algol"
    )))]
    {
        let _ = (cx, expr, codec);
        Err(sim_kernel::Error::HostError(
            "codec-faithful tests require at least one codec feature".to_owned(),
        ))
    }
}

fn failed_report(name: Symbol, detail: String) -> TestReport {
    TestReport::from_result(name, false, Some(detail))
}

fn default_test_codec() -> Symbol {
    Symbol::qualified("codec", "lisp")
}
