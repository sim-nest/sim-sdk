use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use sim_cookbook::{RecipeCard, RecipeStore, ordered_cards, view};
use sim_kernel::{Cx, Ref, Result, Symbol, Value};
use sim_lib_cookbook::{CookbookStoreHandle, seeded_recipe_store, store_symbol};

use crate::runtime::browse::schema::card_v2_from_card_v1;

pub(crate) struct RecipeHelpEntry {
    pub(crate) symbol: Symbol,
    pub(crate) id: String,
    pub(crate) title: String,
}

pub(crate) fn catalog_symbol() -> Symbol {
    Symbol::qualified("cookbook", "catalog")
}

pub(crate) fn root_symbols() -> Vec<Symbol> {
    vec![catalog_symbol()]
}

pub(crate) fn related_recipes(cx: &Cx, subject: &Symbol) -> Result<Vec<RecipeHelpEntry>> {
    with_store(cx, |store| {
        ordered_cards(store)
            .into_iter()
            .filter(|card| recipe_matches_symbol(cx, card, subject))
            .map(|card| RecipeHelpEntry {
                symbol: recipe_symbol(&card.id),
                id: card.id.clone(),
                title: card.title.clone(),
            })
            .collect()
    })
}

pub(crate) fn card_for_symbol(cx: &mut Cx, symbol: &Symbol) -> Result<Option<Value>> {
    if symbol == &catalog_symbol() {
        return catalog_card(cx).map(Some);
    }
    if symbol.namespace.as_deref() == Some("cookbook/book") {
        let book_id = symbol.name.to_string();
        return with_store(cx, view).and_then(|cookbook| book_card(cx, &cookbook.books, &book_id));
    }
    if symbol.namespace.as_deref() == Some("cookbook/chapter") {
        let id = symbol.name.to_string();
        return with_store(cx, view).and_then(|cookbook| chapter_card(cx, &cookbook.books, &id));
    }
    if symbol.namespace.as_deref() == Some("cookbook/recipe") {
        let id = symbol.name.to_string();
        return with_store(cx, |store| store.card(&id).cloned())
            .and_then(|card| recipe_card(cx, card));
    }
    Ok(None)
}

fn catalog_card(cx: &mut Cx) -> Result<Value> {
    let see_also = with_store(cx, |store| {
        view(store)
            .books
            .iter()
            .map(|book| book_symbol(&book.id))
            .collect::<Vec<_>>()
    })?;
    card(
        cx,
        catalog_symbol(),
        Symbol::qualified("cookbook", "catalog"),
        "cookbook view over installed recipe cards",
        &["cookbook:search", "cookbook:run"],
        see_also,
    )
}

fn book_card(
    cx: &mut Cx,
    books: &[sim_cookbook::BookView],
    book_id: &str,
) -> Result<Option<Value>> {
    let Some(book) = books.iter().find(|book| book.id == book_id) else {
        return Ok(None);
    };
    let see_also = book
        .chapters
        .iter()
        .map(|chapter| chapter_symbol(&book.id, &chapter.name))
        .collect();
    let help = if book.summary.is_empty() {
        format!("cookbook book {}", book.title)
    } else {
        book.summary.clone()
    };
    card(
        cx,
        book_symbol(&book.id),
        Symbol::qualified("cookbook", "book"),
        &help,
        &["cookbook:search"],
        see_also,
    )
    .map(Some)
}

fn chapter_card(cx: &mut Cx, books: &[sim_cookbook::BookView], id: &str) -> Result<Option<Value>> {
    for book in books {
        for chapter in &book.chapters {
            if format!("{}/{}", book.id, chapter.name) == id {
                let see_also = chapter
                    .recipes
                    .iter()
                    .map(|recipe| recipe_symbol(&recipe.id))
                    .collect();
                let help = if chapter.summary.is_empty() {
                    format!("cookbook chapter {}", chapter.title)
                } else {
                    chapter.summary.clone()
                };
                return card(
                    cx,
                    chapter_symbol(&book.id, &chapter.name),
                    Symbol::qualified("cookbook", "chapter"),
                    &help,
                    &["cookbook:search"],
                    see_also,
                )
                .map(Some);
            }
        }
    }
    Ok(None)
}

fn recipe_card(cx: &mut Cx, recipe: Option<RecipeCard>) -> Result<Option<Value>> {
    let Some(recipe) = recipe else {
        return Ok(None);
    };
    let mut see_also = Vec::new();
    see_also.push(chapter_symbol(&recipe.book, &recipe.chapter));
    if let Some(next) = with_store(cx, |store| {
        sim_cookbook::next(store, &recipe.id).map(|card| recipe_symbol(&card.id))
    })? {
        see_also.push(next);
    }
    let help = format!("{}: {}", recipe.title, first_line(&recipe.purpose));
    card(
        cx,
        recipe_symbol(&recipe.id),
        Symbol::qualified("cookbook", "recipe"),
        &help,
        &["cookbook:show", "cookbook:run"],
        see_also,
    )
    .map(Some)
}

fn card(
    cx: &mut Cx,
    subject_symbol: Symbol,
    kind: Symbol,
    help: &str,
    ops: &[&str],
    see_also_symbols: Vec<Symbol>,
) -> Result<Value> {
    let subject = Ref::Symbol(subject_symbol);
    let see_also = see_also_symbols
        .into_iter()
        .map(|symbol| cx.factory().symbol(symbol))
        .collect::<Result<Vec<_>>>()?;
    let ops = ops
        .iter()
        .map(|op| cx.factory().string((*op).to_owned()))
        .collect::<Result<Vec<_>>>()?;
    let fallback = cx.factory().table(vec![
        (field("kind"), cx.factory().symbol(kind.clone())?),
        (field("help"), cx.factory().string(help.to_owned())?),
        (
            field("args"),
            cx.factory().symbol(Symbol::qualified("core", "Any"))?,
        ),
        (
            field("result"),
            cx.factory().symbol(Symbol::qualified("core", "Card"))?,
        ),
        (field("tests"), cx.factory().list(Vec::new())?),
        (field("ops"), cx.factory().list(ops)?),
        (field("requires"), cx.factory().list(Vec::new())?),
        (field("see-also"), cx.factory().list(see_also)?),
        (field("shape-known"), cx.factory().bool(true)?),
    ])?;
    let card_v1 = sim_kernel::card::card_for_ref_with_fallback(
        cx,
        subject.clone(),
        Some(fallback),
        Some(kind),
    )?;
    card_v2_from_card_v1(cx, subject, card_v1)
}

fn with_store<T>(cx: &Cx, f: impl FnOnce(&RecipeStore) -> T) -> Result<T> {
    if let Some(store) = shared_store(cx) {
        let store = store.lock().expect("recipe store lock");
        return Ok(f(&store));
    }
    let store = seeded_recipe_store()?;
    Ok(f(&store))
}

fn shared_store(cx: &Cx) -> Option<Arc<Mutex<RecipeStore>>> {
    cx.registry()
        .value_by_symbol(&store_symbol())
        .and_then(|value| value.object().downcast_ref::<CookbookStoreHandle>())
        .map(CookbookStoreHandle::store)
}

fn recipe_matches_symbol(cx: &Cx, card: &RecipeCard, subject: &Symbol) -> bool {
    let mut full_keys = BTreeSet::new();
    full_keys.insert(subject.to_string());
    if let Some(lib) = exporting_lib(cx, subject) {
        full_keys.insert(lib.to_string());
    }
    if card
        .requires
        .iter()
        .any(|required| full_keys.contains(required))
    {
        return true;
    }

    let tags = card
        .tags
        .iter()
        .map(|tag| tag.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    full_keys
        .iter()
        .any(|key| tags.contains(&key.to_ascii_lowercase()) || parts_match(&tags, key))
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

fn parts_match(tags: &BTreeSet<String>, key: &str) -> bool {
    let parts = key.split('/').collect::<Vec<_>>();
    !parts.is_empty()
        && parts
            .iter()
            .all(|part| tags.contains(&part.to_ascii_lowercase()))
}

fn first_line(text: &str) -> &str {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
}

fn book_symbol(book: &str) -> Symbol {
    Symbol::qualified("cookbook/book", book)
}

fn chapter_symbol(book: &str, chapter: &str) -> Symbol {
    Symbol::qualified("cookbook/chapter", format!("{book}/{chapter}"))
}

fn recipe_symbol(id: &str) -> Symbol {
    Symbol::qualified("cookbook/recipe", id)
}

fn field(name: &str) -> Symbol {
    Symbol::new(name)
}
