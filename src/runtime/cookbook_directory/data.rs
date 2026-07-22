macro_rules! cookbook_directory_data {
    ($m:ident) => {
        $m!("list/cons", "Cons list backend", "list-cell", None, || {
            Box::new(crate::list_cell::ConsListLib)
        });
        $m!("list/lazy", "Lazy list backend", "list-lazy", None, || {
            Box::new(crate::list_lazy::LazyListLib)
        });
        $m!(
            "table/hash",
            "Hash table backend",
            "table-hash",
            None,
            || Box::new(crate::table_hash::HashTableLib)
        );
        $m!(
            "table/lazy",
            "Lazy table backend",
            "table-lazy",
            None,
            || Box::new(crate::table_lazy::LazyTableLib)
        );
    };
}
