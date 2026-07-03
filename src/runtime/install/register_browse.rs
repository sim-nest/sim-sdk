use sim_kernel::{Linker, Result, Symbol};

use crate::runtime::browse::{
    args_function, browse_function, browse_neighbors_function, browse_path_function,
    classes_function, codecs_function, coverage_function, eval_policies_function,
    examples_function, export_function, exports_function, facets_function, functions_function,
    help_function, help_object_function, lib_function, lib_tests_function, libs_function,
    macros_function, number_domains_function, result_function, run_tests_function, shapes_function,
    tests_function,
};

use super::register::{CoreBuildCx, link_function, link_two_case_function};

pub(super) fn register_core_browse_functions(
    cx: &mut impl CoreBuildCx,
    linker: &mut Linker<'_>,
) -> Result<()> {
    link_function(cx, linker, Symbol::qualified("core", "libs"), libs_function)?;
    link_function(cx, linker, Symbol::qualified("core", "lib"), lib_function)?;
    link_two_case_function(
        cx,
        linker,
        Symbol::qualified("core", "browse"),
        browse_function,
    )?;
    link_two_case_function(cx, linker, Symbol::new("browse"), browse_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "exports"),
        exports_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "export"),
        export_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "classes"),
        classes_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "functions"),
        functions_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "macros"),
        macros_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "shapes"),
        shapes_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "codecs"),
        codecs_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "number-domains"),
        number_domains_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "eval-policies"),
        eval_policies_function,
    )?;
    link_two_case_function(
        cx,
        linker,
        Symbol::qualified("core", "tests"),
        tests_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "lib-tests"),
        lib_tests_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "run-tests"),
        run_tests_function,
    )?;
    link_function(cx, linker, Symbol::qualified("core", "help"), help_function)?;
    link_function(cx, linker, Symbol::qualified("core", "args"), args_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "result"),
        result_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "examples"),
        examples_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "coverage"),
        coverage_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "facets"),
        facets_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "help-object"),
        help_object_function,
    )?;
    link_two_case_function(
        cx,
        linker,
        Symbol::qualified("core", "browse-neighbors"),
        browse_neighbors_function,
    )?;
    link_two_case_function(
        cx,
        linker,
        Symbol::new("browse-neighbors"),
        browse_neighbors_function,
    )?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "browse-path"),
        browse_path_function,
    )?;
    link_function(cx, linker, Symbol::new("browse-path"), browse_path_function)
}
