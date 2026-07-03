#![cfg(all(feature = "agent", feature = "server-net-http"))]
#![allow(dead_code)]
#![allow(deprecated)]

use std::any::Any;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sim::codec_binary::BinaryCodecLib;
use sim::codec_lisp::LispCodecLib;
use sim::install_agent_lib;
use sim::install_server_lib;
use sim::kernel::{
    Args, Callable, CapabilityName, ClassRef, Cx, DefaultFactory, EagerPolicy, Error, EvalReply,
    Expr, NumberLiteral, Object, QuoteMode, Result, Symbol, Value, eval_fabric_capability,
    eval_remote_capability,
};
use sim::lib_server::{
    Connection, EvalSite, ServerAddress, ServerFrame, eval_request_from_frame,
    server_frame_from_reply,
};
use sim::numbers_f64::F64NumbersLib;
use std::sync::OnceLock;

pub fn cx() -> Cx {
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    let lisp = LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    let binary = BinaryCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&lisp).unwrap();
    cx.load_lib(&binary).unwrap();
    cx.load_lib(&F64NumbersLib::new()).unwrap();
    install_server_lib(&mut cx).unwrap();
    install_agent_lib(&mut cx).unwrap();
    cx.grant(eval_fabric_capability());
    cx.grant(eval_remote_capability());
    cx.grant(CapabilityName::new("agent-spawn"));
    cx.grant(CapabilityName::new("agent-replace"));
    cx.grant(CapabilityName::new("agent-reflect"));
    cx.grant(CapabilityName::new("swarm-launch"));
    cx.grant(CapabilityName::new("agent-drive"));
    cx.grant(CapabilityName::new("mail-read"));
    cx.grant(CapabilityName::new("mail-write"));
    cx.grant(CapabilityName::new("telegram-bot"));
    cx.grant(CapabilityName::new("network"));
    register_math_add(&mut cx);
    cx
}

pub fn keyword(name: &str) -> Expr {
    Expr::Symbol(Symbol::new(format!(":{name}")))
}

pub fn quoted(expr: Expr) -> Expr {
    Expr::Quote {
        mode: QuoteMode::Quote,
        expr: Box::new(expr),
    }
}

pub fn number_expr(value: i64) -> Expr {
    Expr::Number(NumberLiteral {
        domain: Symbol::qualified("numbers", "f64"),
        canonical: value.to_string(),
    })
}

pub fn call_exprs(cx: &mut Cx, symbol: Symbol, exprs: Vec<Expr>) -> Value {
    try_call_exprs(cx, symbol, exprs).unwrap()
}

pub fn try_call_exprs(cx: &mut Cx, symbol: Symbol, exprs: Vec<Expr>) -> Result<Value> {
    cx.call_exprs(cx.resolve_function(&symbol).unwrap(), exprs)
}

pub fn register_value(cx: &mut Cx, symbol: Symbol, value: Value) {
    cx.registry_mut().register_value(symbol, value).unwrap();
}

pub fn register_connection(cx: &mut Cx, symbol: Symbol, connection: Connection) -> Value {
    let value = cx.factory().opaque(Arc::new(connection)).unwrap();
    register_value(cx, symbol, value.clone());
    value
}

pub fn installed_codecs() -> Vec<Symbol> {
    vec![
        Symbol::qualified("codec", "lisp"),
        Symbol::qualified("codec", "binary"),
    ]
}

fn default_codecs() -> &'static Vec<Symbol> {
    static CODECS: OnceLock<Vec<Symbol>> = OnceLock::new();
    CODECS.get_or_init(installed_codecs)
}

pub fn make_connection(site: Arc<dyn EvalSite>) -> Connection {
    Connection::new(
        ServerAddress::Local,
        Symbol::qualified("codec", "lisp"),
        installed_codecs(),
        site,
    )
    .unwrap()
}

pub fn start_server_with_site(
    cx: &mut Cx,
    name: &str,
    address: Expr,
    site_symbol: Symbol,
) -> Value {
    call_exprs(
        cx,
        Symbol::qualified("server", "start"),
        vec![
            keyword("name"),
            Expr::Symbol(Symbol::new(name)),
            keyword("address"),
            address,
            keyword("codec"),
            Expr::Symbol(Symbol::qualified("codec", "lisp")),
            keyword("site"),
            Expr::Symbol(site_symbol),
        ],
    )
}

pub fn in_process_address(name: &str) -> Expr {
    Expr::List(vec![
        Expr::Symbol(Symbol::new("in-process")),
        keyword("thread"),
        Expr::String(name.to_owned()),
    ])
}

pub fn tcp_address_expr(port: u16) -> Expr {
    Expr::List(vec![
        Expr::Symbol(Symbol::new("tcp")),
        keyword("host"),
        Expr::String("127.0.0.1".to_owned()),
        keyword("port"),
        Expr::String(port.to_string()),
    ])
}

pub fn unique_name(prefix: &str) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix}-{nonce}")
}

pub fn unique_id() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

pub fn wait_for(predicate: impl Fn() -> bool) {
    for _ in 0..50 {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("condition was not met in time");
}

pub fn flatten_text(expr: &Expr) -> String {
    match expr {
        Expr::Nil => "nil".to_owned(),
        Expr::Bool(value) => value.to_string(),
        Expr::String(text) => text.clone(),
        Expr::Symbol(symbol) => symbol.to_string(),
        Expr::Number(number) => number.canonical.clone(),
        Expr::List(items) | Expr::Vector(items) | Expr::Set(items) | Expr::Block(items) => {
            items.iter().map(flatten_text).collect::<Vec<_>>().join(" ")
        }
        Expr::Map(entries) => entries
            .iter()
            .map(|(key, value)| format!("{} {}", flatten_text(key), flatten_text(value)))
            .collect::<Vec<_>>()
            .join(" "),
        Expr::Call { operator, args } => std::iter::once(flatten_text(operator))
            .chain(args.iter().map(flatten_text))
            .collect::<Vec<_>>()
            .join(" "),
        Expr::Quote { expr, .. } | Expr::Annotated { expr, .. } => flatten_text(expr),
        Expr::Bytes(bytes) => String::from_utf8_lossy(bytes).into_owned(),
        _ => format!("{expr:?}"),
    }
}

pub fn map_field(expr: &Expr, key: &str) -> Option<Expr> {
    let Expr::Map(entries) = expr else {
        return None;
    };
    entries
        .iter()
        .find_map(|(entry_key, value)| match entry_key {
            Expr::Symbol(symbol) if symbol.name.as_ref() == key => Some(value.clone()),
            _ => None,
        })
}

fn register_math_add(cx: &mut Cx) {
    let value = cx.factory().opaque(Arc::new(MathAddFn)).unwrap();
    register_value(cx, Symbol::qualified("math", "add"), value);
}

#[derive(Clone)]
pub struct TransformSite {
    pub prefix: &'static str,
    pub hops: Arc<Mutex<Vec<u32>>>,
    pub seen: Arc<Mutex<Vec<String>>>,
}

impl EvalSite for TransformSite {
    fn site_kind(&self) -> &'static str {
        "transform"
    }

    fn address(&self) -> &ServerAddress {
        static ADDRESS: ServerAddress = ServerAddress::Local;
        &ADDRESS
    }

    fn codecs(&self) -> &[Symbol] {
        default_codecs()
    }

    fn answer(&self, cx: &mut Cx, frame: ServerFrame) -> Result<ServerFrame> {
        let expr = eval_request_from_frame(cx, &frame)?.expr;
        self.hops.lock().unwrap().push(frame.envelope.hop);
        self.seen.lock().unwrap().push(self.prefix.to_owned());
        let output = Expr::String(format!("{}{}", self.prefix, flatten_text(&expr)));
        reply_expr(cx, &frame, output)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
pub struct FixedReplySite {
    pub reply: Expr,
}

impl EvalSite for FixedReplySite {
    fn site_kind(&self) -> &'static str {
        "fixed-reply"
    }

    fn address(&self) -> &ServerAddress {
        static ADDRESS: ServerAddress = ServerAddress::Local;
        &ADDRESS
    }

    fn codecs(&self) -> &[Symbol] {
        default_codecs()
    }

    fn answer(&self, cx: &mut Cx, frame: ServerFrame) -> Result<ServerFrame> {
        reply_expr(cx, &frame, self.reply.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
pub struct DriverPersonaSite;

impl EvalSite for DriverPersonaSite {
    fn site_kind(&self) -> &'static str {
        "driver-persona"
    }

    fn address(&self) -> &ServerAddress {
        static ADDRESS: ServerAddress = ServerAddress::Local;
        &ADDRESS
    }

    fn codecs(&self) -> &[Symbol] {
        default_codecs()
    }

    fn answer(&self, cx: &mut Cx, frame: ServerFrame) -> Result<ServerFrame> {
        let expr = eval_request_from_frame(cx, &frame)?.expr;
        let output = map_field(&expr, "output")
            .map(|value| flatten_text(&value))
            .unwrap_or_default();
        let reply = if output.contains("42") {
            Expr::Map(vec![(Expr::Symbol(Symbol::new("done")), Expr::Bool(true))])
        } else {
            Expr::String("(+ 40 2)".to_owned())
        };
        reply_expr(cx, &frame, reply)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
pub struct RecordingPersonaSite {
    pub prefix: &'static str,
    pub replies: Arc<Mutex<Vec<String>>>,
}

impl EvalSite for RecordingPersonaSite {
    fn site_kind(&self) -> &'static str {
        "recording-persona"
    }

    fn address(&self) -> &ServerAddress {
        static ADDRESS: ServerAddress = ServerAddress::Local;
        &ADDRESS
    }

    fn codecs(&self) -> &[Symbol] {
        default_codecs()
    }

    fn answer(&self, cx: &mut Cx, frame: ServerFrame) -> Result<ServerFrame> {
        let expr = eval_request_from_frame(cx, &frame)?.expr;
        let reply = format!("{}{}", self.prefix, flatten_text(&expr));
        self.replies.lock().unwrap().push(reply.clone());
        reply_expr(cx, &frame, Expr::String(reply))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
pub struct CaptureReplySite {
    pub seen: Arc<Mutex<Vec<Expr>>>,
    pub reply: Expr,
}

impl EvalSite for CaptureReplySite {
    fn site_kind(&self) -> &'static str {
        "capture-reply"
    }

    fn address(&self) -> &ServerAddress {
        static ADDRESS: ServerAddress = ServerAddress::Local;
        &ADDRESS
    }

    fn codecs(&self) -> &[Symbol] {
        default_codecs()
    }

    fn answer(&self, cx: &mut Cx, frame: ServerFrame) -> Result<ServerFrame> {
        let expr = eval_request_from_frame(cx, &frame)?.expr;
        self.seen.lock().unwrap().push(expr);
        reply_expr(cx, &frame, self.reply.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct MathAddFn;

impl Object for MathAddFn {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok("#<function math/add>".to_owned())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl sim_kernel::ObjectCompat for MathAddFn {
    fn class(&self, cx: &mut Cx) -> Result<ClassRef> {
        cx.factory().class_stub(
            sim::kernel::CORE_FUNCTION_CLASS_ID,
            Symbol::qualified("core", "Function"),
        )
    }
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

impl Callable for MathAddFn {
    fn call(&self, cx: &mut Cx, args: Args) -> Result<Value> {
        let mut sum = 0.0_f64;
        for value in args.values() {
            let Expr::Number(number) = value.object().as_expr(cx)? else {
                return Err(Error::Eval("math/add expects number args".to_owned()));
            };
            sum += number.canonical.parse::<f64>().unwrap();
        }
        cx.factory()
            .number_literal(Symbol::qualified("numbers", "f64"), format!("{sum:.0}"))
    }
}

pub fn reply_expr(cx: &mut Cx, frame: &ServerFrame, expr: Expr) -> Result<ServerFrame> {
    server_frame_from_reply(
        cx,
        &frame.codec,
        EvalReply {
            value: cx.factory().expr(expr)?,
            diagnostics: Vec::new(),
            trace: None,
        },
        frame.envelope.consistency,
    )
}

pub fn lower_repl_like(expr: Expr) -> Expr {
    match expr {
        Expr::List(items) => {
            let mut items = items.into_iter();
            let Some(operator) = items.next() else {
                return Expr::List(Vec::new());
            };
            let operator = match operator {
                Expr::Symbol(symbol) if symbol.name.as_ref() == "+" => {
                    Expr::Symbol(Symbol::qualified("math", "add"))
                }
                other => other,
            };
            Expr::Call {
                operator: Box::new(operator),
                args: items.collect(),
            }
        }
        other => other,
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn normalize_server_reflect(text: String) -> String {
    let mut out = Vec::new();
    let mut prev = "";
    for token in text.split_whitespace() {
        if prev == "server" && token.parse::<u64>().is_ok() {
            continue;
        }
        if prev == "id" || prev == "uptime-ms" {
            prev = token;
            continue;
        }
        out.push(token.to_owned());
        prev = token;
    }
    out.join(" ")
}

pub fn is_socket_permission_error(error: &sim::kernel::Error) -> bool {
    matches!(
        error,
        sim::kernel::Error::HostError(message)
            if message.contains("PermissionDenied") || message.contains("Operation not permitted")
    )
}
