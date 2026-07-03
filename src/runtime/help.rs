use sim_kernel::{
    Claim, ClaimPattern, Cx, Datum, DatumStore, Demand, Error, Ref, Result, ShapeRef, Symbol, Value,
};

use crate::{
    classes::NativeClass,
    functions::FunctionObject,
    macros::MacroObject,
    runtime::browse::{
        predicates::help_doc_predicate,
        reflection::{self, AuthoredHelp},
        schema::HelpBuilder,
    },
    shapes::shape_value,
};

#[derive(Clone)]
pub(crate) struct HelpDocument {
    subject: Symbol,
    kind: Symbol,
    summary: String,
    detail: String,
    exported_by: Option<Symbol>,
    args: Option<ShapeRef>,
    result: Option<ShapeRef>,
    demand: Vec<Demand>,
    tests: Vec<Symbol>,
    capabilities: Vec<String>,
    see_also: Vec<Symbol>,
    legacy_summary: Value,
}

impl HelpDocument {
    pub(crate) fn help_value(&self, cx: &mut Cx) -> Result<Value> {
        let subject = cx.factory().symbol(self.subject.clone())?;
        let capabilities = self
            .capabilities
            .iter()
            .cloned()
            .map(|capability| cx.factory().string(capability))
            .collect::<Result<Vec<_>>>()?;
        let demand = self
            .demand
            .iter()
            .map(|demand| cx.factory().symbol(Symbol::new(demand_name(*demand))))
            .collect::<Result<Vec<_>>>()?;
        let see_also = self
            .see_also
            .iter()
            .cloned()
            .map(|symbol| cx.factory().symbol(symbol))
            .collect::<Result<Vec<_>>>()?;

        let mut builder = HelpBuilder::new(subject);
        builder.kind = self.kind.clone();
        builder.summary = self.summary.clone();
        builder.detail = self.detail.clone();
        builder.stability = Symbol::new("unknown");
        builder.capabilities = capabilities;
        builder.demand = demand;
        builder.see_also = see_also;
        if let Some(exported_by) = &self.exported_by {
            builder.exported_by = Some(cx.factory().symbol(exported_by.clone())?);
        }
        builder.build(cx)
    }

    pub(crate) fn fallback_value(&self, cx: &mut Cx) -> Result<Value> {
        let mut entries = vec![
            (
                Symbol::new("subject"),
                cx.factory().symbol(self.subject.clone())?,
            ),
            (Symbol::new("kind"), cx.factory().symbol(self.kind.clone())?),
            (
                Symbol::new("purpose"),
                cx.factory().string(self.summary.clone())?,
            ),
            (Symbol::new("summary"), self.legacy_summary.clone()),
            (
                Symbol::new("exported-by"),
                match &self.exported_by {
                    Some(symbol) => cx.factory().symbol(symbol.clone())?,
                    None => cx.factory().nil()?,
                },
            ),
            (
                Symbol::new("tests"),
                cx.factory().list(
                    self.tests
                        .iter()
                        .cloned()
                        .map(|symbol| cx.factory().symbol(symbol))
                        .collect::<Result<Vec<_>>>()?,
                )?,
            ),
        ];
        if let Some(args) = &self.args {
            entries.push((Symbol::new("args"), args.clone()));
        }
        if let Some(result) = &self.result {
            entries.push((Symbol::new("result"), result.clone()));
        }
        if !self.demand.is_empty() {
            entries.push((
                Symbol::new("demand"),
                cx.factory().list(
                    self.demand
                        .iter()
                        .map(|demand| cx.factory().symbol(Symbol::new(demand_name(*demand))))
                        .collect::<Result<Vec<_>>>()?,
                )?,
            ));
        }
        if !self.detail.is_empty() {
            entries.push((
                Symbol::new("detail"),
                cx.factory().string(self.detail.clone())?,
            ));
        }
        if !self.see_also.is_empty() {
            entries.push((
                Symbol::new("see-also"),
                cx.factory().list(
                    self.see_also
                        .iter()
                        .cloned()
                        .map(|symbol| cx.factory().symbol(symbol))
                        .collect::<Result<Vec<_>>>()?,
                )?,
            ));
        }
        cx.factory().table(entries)
    }

    pub(crate) fn publish_claims(&self, cx: &mut Cx) -> Result<()> {
        let value = self.help_value(cx)?;
        let datum = Datum::try_from(value.object().as_expr(cx)?)?;
        let claim = Claim::content_object(
            cx.datum_store_mut(),
            Ref::Symbol(self.subject.clone()),
            help_doc_predicate(),
            datum,
        )?;
        cx.insert_fact(claim)?;
        self.publish_shape_claims(cx)?;
        Ok(())
    }

    fn publish_shape_claims(&self, cx: &mut Cx) -> Result<()> {
        let subject = Ref::Symbol(self.subject.clone());
        let args_known = publish_shape_claim(
            cx,
            &subject,
            sim_kernel::card::card_args_predicate(),
            self.args.as_ref(),
        )?;
        let result_known = publish_shape_claim(
            cx,
            &subject,
            sim_kernel::card::card_result_predicate(),
            self.result.as_ref(),
        )?;
        let claim = Claim::content_object(
            cx.datum_store_mut(),
            subject,
            sim_kernel::card::card_shape_known_predicate(),
            Datum::Bool(args_known && result_known),
        )?;
        cx.insert_fact(claim)?;
        Ok(())
    }
}

pub(crate) fn build_help(cx: &mut Cx, subject: &Symbol) -> Result<HelpDocument> {
    let mut help = discovered_help(cx, subject)?;
    if let Some(authored) = reflection::authored_help(subject) {
        apply_authored_help(&mut help, authored);
    }
    #[cfg(feature = "cookbook")]
    apply_recipe_help(cx, &mut help)?;
    Ok(help)
}

fn discovered_help(cx: &mut Cx, subject: &Symbol) -> Result<HelpDocument> {
    if let Ok(value) = cx.resolve_function(subject) {
        let callable = value
            .object()
            .as_callable()
            .ok_or_else(|| Error::HostError("function value did not expose Callable".to_owned()))?;
        let function = value.object().downcast_ref::<FunctionObject>();
        let summary = function
            .map(|function| format!("function with {} overload case(s)", function.cases.len()))
            .unwrap_or_else(|| "callable function surface".to_owned());
        return Ok(HelpDocument {
            subject: subject.clone(),
            kind: core_kind("function"),
            summary,
            detail: String::new(),
            exported_by: exporting_lib(cx, subject),
            args: callable.browse_args_shape(cx)?,
            result: callable.browse_result_shape(cx)?,
            demand: function
                .map(FunctionObject::declared_demands)
                .unwrap_or_default(),
            tests: tests_for_subject(cx, subject),
            capabilities: capabilities_for_subject(cx, subject)?,
            see_also: Vec::new(),
            legacy_summary: value.object().as_table(cx)?,
        });
    }
    if let Ok(value) = cx.resolve_class(subject) {
        let class = value.object().as_class();
        let native = value.object().downcast_ref::<NativeClass>();
        let summary = match (native, class) {
            (Some(class), _) => format!(
                "class with {} member field(s)",
                class.member_functions().len()
            ),
            (None, Some(class)) => format!("class {}", class.symbol()),
            (None, None) => format!("class {subject}"),
        };
        return Ok(HelpDocument {
            subject: subject.clone(),
            kind: core_kind("class"),
            summary,
            detail: String::new(),
            exported_by: exporting_lib(cx, subject),
            args: match class {
                Some(class) => Some(class.constructor_shape(cx)?),
                None => None,
            },
            result: match class {
                Some(class) => Some(class.instance_shape(cx)?),
                None => None,
            },
            demand: native
                .map(|class| class.constructor.declared_demands())
                .unwrap_or_default(),
            tests: tests_for_subject(cx, subject),
            capabilities: capabilities_for_subject(cx, subject)?,
            see_also: Vec::new(),
            legacy_summary: value.object().as_table(cx)?,
        });
    }
    if let Ok(value) = cx.resolve_macro(subject) {
        let mac = value
            .object()
            .downcast_ref::<MacroObject>()
            .ok_or_else(|| Error::HostError("macro value was not a MacroObject".to_owned()))?;
        return Ok(HelpDocument {
            subject: subject.clone(),
            kind: core_kind("macro"),
            summary: "macro syntax surface".to_owned(),
            detail: String::new(),
            exported_by: exporting_lib(cx, subject),
            args: Some(shape_value(
                Symbol::qualified(subject.to_string(), "syntax-shape"),
                mac.syntax_shape(),
            )),
            result: None,
            demand: Vec::new(),
            tests: tests_for_subject(cx, subject),
            capabilities: capabilities_for_subject(cx, subject)?,
            see_also: Vec::new(),
            legacy_summary: value.object().as_table(cx)?,
        });
    }
    if let Ok(value) = cx.resolve_shape(subject) {
        return Ok(HelpDocument {
            subject: subject.clone(),
            kind: core_kind("shape"),
            summary: "shape contract".to_owned(),
            detail: String::new(),
            exported_by: exporting_lib(cx, subject),
            args: None,
            result: None,
            demand: Vec::new(),
            tests: tests_for_subject(cx, subject),
            capabilities: capabilities_for_subject(cx, subject)?,
            see_also: Vec::new(),
            legacy_summary: value.object().as_table(cx)?,
        });
    }
    if let Ok(value) = cx.resolve_codec(subject) {
        return Ok(HelpDocument {
            subject: subject.clone(),
            kind: core_kind("codec"),
            summary: "codec runtime with encoder and decoder surfaces".to_owned(),
            detail: String::new(),
            exported_by: exporting_lib(cx, subject),
            args: None,
            result: None,
            demand: Vec::new(),
            tests: tests_for_subject(cx, subject),
            capabilities: capabilities_for_subject(cx, subject)?,
            see_also: Vec::new(),
            legacy_summary: value.object().as_table(cx)?,
        });
    }
    if let Ok(value) = cx.resolve_number_domain(subject) {
        return Ok(HelpDocument {
            subject: subject.clone(),
            kind: core_kind("number-domain"),
            summary: "number domain and dispatch metadata".to_owned(),
            detail: String::new(),
            exported_by: exporting_lib(cx, subject),
            args: None,
            result: None,
            demand: Vec::new(),
            tests: tests_for_subject(cx, subject),
            capabilities: capabilities_for_subject(cx, subject)?,
            see_also: Vec::new(),
            legacy_summary: value.object().as_table(cx)?,
        });
    }
    if let Some(loaded) = cx.registry().lib(subject).cloned() {
        return Ok(HelpDocument {
            subject: subject.clone(),
            kind: core_kind("lib"),
            summary: "loaded library manifest".to_owned(),
            detail: String::new(),
            exported_by: Some(loaded.manifest.id.clone()),
            args: None,
            result: None,
            demand: Vec::new(),
            tests: cx.registry().tests_for_lib(subject).unwrap_or(&[]).to_vec(),
            capabilities: capabilities_for_subject(cx, subject)?,
            see_also: Vec::new(),
            legacy_summary: crate::runtime::browse::loaded_lib_value(cx, &loaded)?,
        });
    }
    if let Some(test) = cx.registry().registered_test(subject).cloned() {
        return Ok(HelpDocument {
            subject: subject.clone(),
            kind: core_kind("test"),
            summary: "registered runtime test".to_owned(),
            detail: String::new(),
            exported_by: Some(test.lib.clone()),
            args: None,
            result: None,
            demand: Vec::new(),
            tests: vec![test.symbol.clone()],
            capabilities: capabilities_for_subject(cx, subject)?,
            see_also: Vec::new(),
            legacy_summary: test.test.describe(cx)?,
        });
    }
    if let Ok(value) = cx.resolve_value(subject) {
        return Ok(HelpDocument {
            subject: subject.clone(),
            kind: core_kind("value"),
            summary: "registered runtime value".to_owned(),
            detail: String::new(),
            exported_by: exporting_lib(cx, subject),
            args: None,
            result: None,
            demand: Vec::new(),
            tests: tests_for_subject(cx, subject),
            capabilities: capabilities_for_subject(cx, subject)?,
            see_also: Vec::new(),
            legacy_summary: value.object().as_table(cx)?,
        });
    }
    Err(Error::UnknownSymbol {
        symbol: subject.clone(),
    })
}

fn apply_authored_help(help: &mut HelpDocument, authored: AuthoredHelp) {
    help.kind = core_kind(authored.kind);
    help.summary = authored.summary.to_owned();
    help.detail = authored.detail.to_owned();
    help.see_also = authored.see_also_symbols();
}

#[cfg(feature = "cookbook")]
fn apply_recipe_help(cx: &Cx, help: &mut HelpDocument) -> Result<()> {
    let recipes = crate::runtime::cookbook_discovery::related_recipes(cx, &help.subject)?;
    if recipes.is_empty() {
        return Ok(());
    }
    if !help.detail.is_empty() {
        help.detail.push_str("\n\n");
    }
    help.detail.push_str("Recipes:");
    for recipe in recipes {
        help.detail
            .push_str(&format!("\n- {} - {}", recipe.id, recipe.title));
        if !help.see_also.iter().any(|symbol| symbol == &recipe.symbol) {
            help.see_also.push(recipe.symbol);
        }
    }
    Ok(())
}

fn exporting_lib(cx: &Cx, subject: &Symbol) -> Option<Symbol> {
    cx.registry()
        .libs()
        .iter()
        .find(|loaded| {
            loaded
                .exports
                .iter()
                .any(|export| &export.symbol == subject)
        })
        .map(|loaded| loaded.manifest.id.clone())
}

fn tests_for_subject(cx: &Cx, subject: &Symbol) -> Vec<Symbol> {
    cx.registry()
        .tests()
        .values()
        .filter(|registered| registered.subjects.iter().any(|item| item == subject))
        .map(|registered| registered.symbol.clone())
        .collect()
}

fn capabilities_for_subject(cx: &Cx, subject: &Symbol) -> Result<Vec<String>> {
    let mut capabilities = cx
        .query_facts(ClaimPattern {
            subject: Some(Ref::Symbol(subject.clone())),
            predicate: Some(sim_kernel::card::card_requires_predicate()),
            object: None,
            include_revoked: false,
        })?
        .into_iter()
        .filter_map(|claim| capability_name(cx, &claim.object))
        .collect::<Vec<_>>();
    capabilities.sort();
    capabilities.dedup();
    Ok(capabilities)
}

fn capability_name(cx: &Cx, reference: &Ref) -> Option<String> {
    match reference {
        Ref::Symbol(symbol) => Some(symbol.to_string()),
        Ref::Content(id) => match cx.datum_store().get(id).ok().flatten() {
            Some(Datum::String(value)) => Some(value.clone()),
            Some(Datum::Symbol(symbol)) => Some(symbol.to_string()),
            _ => None,
        },
        Ref::Handle(_) | Ref::Coord(_) => None,
    }
}

fn publish_shape_claim(
    cx: &mut Cx,
    subject: &Ref,
    predicate: Symbol,
    shape: Option<&ShapeRef>,
) -> Result<bool> {
    let known = shape.and_then(shape_claim_ref);
    let object = known
        .clone()
        .unwrap_or_else(|| Ref::Symbol(Symbol::qualified("core", "Any")));
    cx.insert_fact(Claim::public(subject.clone(), predicate, object))?;
    Ok(known.is_some())
}

fn shape_claim_ref(shape: &ShapeRef) -> Option<Ref> {
    shape
        .object()
        .as_shape()
        .and_then(|shape| shape.symbol())
        .map(Ref::Symbol)
}

fn demand_name(demand: Demand) -> &'static str {
    match demand {
        Demand::Never => "never",
        Demand::Value => "value",
        Demand::Expr => "expr",
        Demand::Bool => "bool",
        Demand::Class(_) => "class",
        Demand::Shape(_) => "shape",
    }
}

fn core_kind(name: &str) -> Symbol {
    Symbol::qualified("core", name)
}
