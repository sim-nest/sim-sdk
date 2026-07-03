#![cfg(all(feature = "agent", feature = "server-net-http"))]
#![allow(deprecated)]

#[path = "server_agent_e2e_support.rs"]
mod support;

use std::any::Any;
use std::sync::{Arc, Mutex};

use sim::codec::{Input, decode_with_codec};
use sim::kernel::{Args, Callable, ClassRef, Expr, Object, ReadPolicy, Result, Symbol, Value};
use sim::lib_server::{Connection, FrameKind, Server, ServerAddress, ServerFrame};

use support::{
    RecordingPersonaSite, TransformSite, call_exprs, cx, flatten_text, in_process_address,
    is_socket_permission_error, keyword, lower_repl_like, make_connection,
    normalize_server_reflect, now_ms, number_expr, quoted, register_connection, register_value,
    start_server_with_site, tcp_address_expr, try_call_exprs, unique_id,
};

#[test]
fn r21_pipeline_as_address_transforms_through_each_hop_in_order() {
    let mut cx = cx();
    let hops_a = Arc::new(Mutex::new(Vec::new()));
    let hops_b = Arc::new(Mutex::new(Vec::new()));
    let hops_c = Arc::new(Mutex::new(Vec::new()));
    let seen_a = Arc::new(Mutex::new(Vec::new()));
    let seen_b = Arc::new(Mutex::new(Vec::new()));
    let seen_c = Arc::new(Mutex::new(Vec::new()));

    register_connection(
        &mut cx,
        Symbol::qualified("test", "pipe-a"),
        make_connection(Arc::new(TransformSite {
            prefix: "a:",
            hops: hops_a.clone(),
            seen: seen_a.clone(),
        })),
    );
    register_connection(
        &mut cx,
        Symbol::qualified("test", "pipe-b"),
        make_connection(Arc::new(TransformSite {
            prefix: "b:",
            hops: hops_b.clone(),
            seen: seen_b.clone(),
        })),
    );
    register_connection(
        &mut cx,
        Symbol::qualified("test", "pipe-c"),
        make_connection(Arc::new(TransformSite {
            prefix: "c:",
            hops: hops_c.clone(),
            seen: seen_c.clone(),
        })),
    );

    let a_name = unique_id().to_string();
    let b_name = unique_id().to_string();
    let c_name = unique_id().to_string();

    start_server_with_site(
        &mut cx,
        &a_name,
        in_process_address(&a_name),
        Symbol::qualified("test", "pipe-a"),
    );
    start_server_with_site(
        &mut cx,
        &b_name,
        in_process_address(&b_name),
        Symbol::qualified("test", "pipe-b"),
    );
    start_server_with_site(
        &mut cx,
        &c_name,
        in_process_address(&c_name),
        Symbol::qualified("test", "pipe-c"),
    );

    let pipeline = Expr::List(vec![
        Expr::Symbol(Symbol::new("pipeline")),
        in_process_address(&a_name),
        in_process_address(&b_name),
        in_process_address(&c_name),
    ]);
    let connection = call_exprs(
        &mut cx,
        Symbol::qualified("server", "connect"),
        vec![quoted(pipeline)],
    );
    register_value(&mut cx, Symbol::qualified("test", "pipeline"), connection);

    let reply = call_exprs(
        &mut cx,
        Symbol::qualified("server", "request"),
        vec![
            Expr::Symbol(Symbol::qualified("test", "pipeline")),
            Expr::String("seed".to_owned()),
        ],
    );
    assert_eq!(
        reply.object().as_expr(&mut cx).unwrap(),
        Expr::String("c:b:a:seed".to_owned())
    );
    assert_eq!(*hops_a.lock().unwrap(), vec![1]);
    assert_eq!(*hops_b.lock().unwrap(), vec![2]);
    assert_eq!(*hops_c.lock().unwrap(), vec![3]);
}

#[test]
fn r21_self_restarting_server_round_trips_lisp_and_reflection() {
    let mut cx = cx();
    let server = call_exprs(
        &mut cx,
        Symbol::qualified("server", "start"),
        vec![
            keyword("name"),
            Expr::Symbol(Symbol::new("restartable")),
            keyword("address"),
            Expr::Symbol(Symbol::new("local")),
            keyword("codec"),
            Expr::Symbol(Symbol::qualified("codec", "lisp")),
        ],
    );
    register_value(&mut cx, Symbol::qualified("test", "srv"), server.clone());

    let original = call_exprs(
        &mut cx,
        Symbol::qualified("server", "reflect"),
        vec![Expr::Symbol(Symbol::qualified("test", "srv"))],
    )
    .object()
    .as_expr(&mut cx)
    .unwrap();
    let snapshot = call_exprs(
        &mut cx,
        Symbol::qualified("server", "lisp"),
        vec![Expr::Symbol(Symbol::qualified("test", "srv"))],
    )
    .object()
    .as_expr(&mut cx)
    .unwrap();
    call_exprs(
        &mut cx,
        Symbol::qualified("server", "stop"),
        vec![Expr::Symbol(Symbol::qualified("test", "srv"))],
    );

    let rebuilt = cx.eval_expr(snapshot).unwrap();
    register_value(&mut cx, Symbol::qualified("test", "rebuilt"), rebuilt);
    let rebuilt_reflect = call_exprs(
        &mut cx,
        Symbol::qualified("server", "reflect"),
        vec![Expr::Symbol(Symbol::qualified("test", "rebuilt"))],
    )
    .object()
    .as_expr(&mut cx)
    .unwrap();

    let original_text = normalize_server_reflect(flatten_text(&original));
    let rebuilt_text = normalize_server_reflect(flatten_text(&rebuilt_reflect));
    assert_eq!(original_text, rebuilt_text);
}

#[test]
fn r21_mail_driven_repl_records_the_evaluated_reply() {
    let mut cx = cx();
    let delivered = Arc::new(Mutex::new(Vec::<String>::new()));

    let record_mail = cx
        .factory()
        .opaque(Arc::new(MailResultFn {
            delivered: delivered.clone(),
        }))
        .unwrap();
    let decode_mail = cx.factory().opaque(Arc::new(MailDecodeFn)).unwrap();
    register_value(
        &mut cx,
        Symbol::qualified("test", "mail-result"),
        record_mail,
    );
    register_value(
        &mut cx,
        Symbol::qualified("test", "mail-decode"),
        decode_mail.clone(),
    );

    let server = call_exprs(
        &mut cx,
        Symbol::qualified("server", "start"),
        vec![
            keyword("name"),
            Expr::Symbol(Symbol::new("mail-repl")),
            keyword("address"),
            Expr::Symbol(Symbol::new("local")),
            keyword("codec"),
            Expr::Symbol(Symbol::qualified("codec", "lisp")),
        ],
    );
    let server = server.object().downcast_ref::<Server>().unwrap().clone();

    let decoded = cx
        .call_value(
            decode_mail,
            Args::new(vec![cx.factory().string("(+ 1 2)".to_owned()).unwrap()]),
        )
        .unwrap()
        .object()
        .as_expr(&mut cx)
        .unwrap();
    let frame = ServerFrame::from_expr(
        &mut cx,
        Symbol::qualified("codec", "lisp"),
        FrameKind::Trigger {
            source: Symbol::new("imap"),
            when_ms: now_ms(),
        },
        &decoded,
        sim::kernel::Consistency::LocalFirst,
        Vec::new(),
        false,
    )
    .unwrap();
    server.deliver_trigger_frame(&mut cx, frame).unwrap();

    assert_eq!(delivered.lock().unwrap().as_slice(), ["3"]);
}

#[test]
fn r21_telegram_driven_agent_replies_through_the_trigger_path() {
    let mut cx = cx();
    let replies = Arc::new(Mutex::new(Vec::<String>::new()));

    register_connection(
        &mut cx,
        Symbol::qualified("test", "telegram-persona"),
        make_connection(Arc::new(RecordingPersonaSite {
            prefix: "agent:",
            replies: replies.clone(),
        })),
    );
    let agent = call_exprs(
        &mut cx,
        Symbol::qualified("agent", "make"),
        vec![
            keyword("name"),
            Expr::Symbol(Symbol::new("telebot")),
            keyword("persona"),
            Expr::Symbol(Symbol::qualified("test", "telegram-persona")),
        ],
    );
    register_value(&mut cx, Symbol::qualified("test", "telebot"), agent.clone());
    let server = call_exprs(
        &mut cx,
        Symbol::qualified("server", "start"),
        vec![
            keyword("name"),
            Expr::Symbol(Symbol::new("telebot-server")),
            keyword("address"),
            Expr::Symbol(Symbol::new("local")),
            keyword("codec"),
            Expr::Symbol(Symbol::qualified("codec", "lisp")),
        ],
    );
    let server = server.object().downcast_ref::<Server>().unwrap().clone();

    let frame = ServerFrame::from_expr(
        &mut cx,
        Symbol::qualified("codec", "lisp"),
        FrameKind::Trigger {
            source: Symbol::new("telegram"),
            when_ms: now_ms(),
        },
        &Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::qualified("agent", "call"))),
            args: vec![
                Expr::Symbol(Symbol::qualified("test", "telebot")),
                Expr::String("hello telegram".to_owned()),
            ],
        },
        sim::kernel::Consistency::LocalFirst,
        Vec::new(),
        false,
    )
    .unwrap();
    server.deliver_trigger_frame(&mut cx, frame).unwrap();

    assert_eq!(replies.lock().unwrap().as_slice(), ["agent:hello telegram"]);
}

#[test]
fn r21_open_claw_dispatch_flows_through_every_stage() {
    let mut cx = cx();
    for (name, prefix) in [
        ("gateway", "gateway:"),
        ("inbox", "inbox:"),
        ("dispatcher", "dispatcher:"),
        ("tool", "tool:"),
        ("persona", "persona:"),
        ("outbound", "outbound:"),
    ] {
        register_connection(
            &mut cx,
            Symbol::qualified("test", name),
            make_connection(Arc::new(TransformSite {
                prefix,
                hops: Arc::new(Mutex::new(Vec::new())),
                seen: Arc::new(Mutex::new(Vec::new())),
            })),
        );
    }

    let open_claw = call_exprs(
        &mut cx,
        Symbol::qualified("topology", "open-claw"),
        vec![
            keyword("steps"),
            quoted(Expr::List(vec![
                Expr::Symbol(Symbol::qualified("test", "gateway")),
                Expr::Symbol(Symbol::qualified("test", "inbox")),
                Expr::Symbol(Symbol::qualified("test", "dispatcher")),
                Expr::Symbol(Symbol::qualified("test", "tool")),
                Expr::Symbol(Symbol::qualified("test", "persona")),
                Expr::Symbol(Symbol::qualified("test", "outbound")),
            ])),
        ],
    );
    let reply = open_claw
        .object()
        .downcast_ref::<Connection>()
        .unwrap()
        .request(&mut cx, Expr::String("handle".to_owned()), None, Vec::new())
        .unwrap()
        .object()
        .as_expr(&mut cx)
        .unwrap();
    assert_eq!(
        reply,
        Expr::String("outbound:persona:tool:dispatcher:inbox:gateway:handle".to_owned())
    );
}

#[test]
fn r21_cross_process_realize_round_trips_over_real_tcp() {
    for _ in 0..5 {
        let mut cx = cx();
        let server = match try_call_exprs(
            &mut cx,
            Symbol::qualified("server", "start"),
            vec![
                keyword("name"),
                Expr::Symbol(Symbol::new("tcp-r21")),
                keyword("address"),
                tcp_address_expr(0),
                keyword("codec"),
                Expr::Symbol(Symbol::qualified("codec", "lisp")),
            ],
        ) {
            Ok(server) => server,
            Err(error) => {
                if is_socket_permission_error(&error) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
                continue;
            }
        };
        let server_ref = server.object().downcast_ref::<Server>().unwrap();
        let ServerAddress::Tcp { port, .. } = server_ref.address() else {
            panic!("expected tcp server address");
        };

        let result = try_call_exprs(
            &mut cx,
            Symbol::qualified("server", "realize"),
            vec![
                Expr::Call {
                    operator: Box::new(Expr::Symbol(Symbol::qualified("math", "add"))),
                    args: vec![number_expr(40), number_expr(2)],
                },
                keyword("on"),
                quoted(tcp_address_expr(*port)),
            ],
        );
        match result {
            Ok(realized) => {
                assert_eq!(realized.object().as_expr(&mut cx).unwrap(), number_expr(42));
                return;
            }
            Err(error) => {
                if is_socket_permission_error(&error) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        }
    }
    panic!("real tcp realize did not succeed after retries");
}

struct MailDecodeFn;

impl Object for MailDecodeFn {
    fn display(&self, _cx: &mut sim::kernel::Cx) -> Result<String> {
        Ok("#<function test/mail-decode>".to_owned())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl sim_kernel::ObjectCompat for MailDecodeFn {
    fn class(&self, cx: &mut sim::kernel::Cx) -> Result<ClassRef> {
        cx.factory().class_stub(
            sim::kernel::CORE_FUNCTION_CLASS_ID,
            Symbol::qualified("core", "Function"),
        )
    }
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

impl Callable for MailDecodeFn {
    fn call(&self, cx: &mut sim::kernel::Cx, args: Args) -> Result<Value> {
        let body = args.values()[0].object().as_expr(cx)?;
        let Expr::String(text) = body else {
            panic!("mail decode expects a string body");
        };
        let parsed = decode_with_codec(
            cx,
            &Symbol::qualified("codec", "lisp"),
            Input::Text(text),
            ReadPolicy::default(),
        )?;
        cx.factory().expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::qualified("test", "mail-result"))),
            args: vec![lower_repl_like(parsed)],
        })
    }
}

struct MailResultFn {
    delivered: Arc<Mutex<Vec<String>>>,
}

impl Object for MailResultFn {
    fn display(&self, _cx: &mut sim::kernel::Cx) -> Result<String> {
        Ok("#<function test/mail-result>".to_owned())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl sim_kernel::ObjectCompat for MailResultFn {
    fn class(&self, cx: &mut sim::kernel::Cx) -> Result<ClassRef> {
        cx.factory().class_stub(
            sim::kernel::CORE_FUNCTION_CLASS_ID,
            Symbol::qualified("core", "Function"),
        )
    }
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

impl Callable for MailResultFn {
    fn call(&self, cx: &mut sim::kernel::Cx, args: Args) -> Result<Value> {
        self.delivered
            .lock()
            .unwrap()
            .push(flatten_text(&args.values()[0].object().as_expr(cx)?));
        cx.factory().nil()
    }
}
