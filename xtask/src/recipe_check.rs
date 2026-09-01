use std::process::Command;

pub fn run(args: &[String]) -> Result<(), String> {
    let program = args.first().map(String::as_str).unwrap_or("xtask");
    if args.len() != 2 {
        return Err(format!("usage: {program} check-recipes"));
    }

    let cookbook_status = Command::new("cargo")
        .args([
            "test",
            "--test",
            "cookbook",
            "cookbook_recipe_gate_runs_every_seeded_sdk_recipe",
            "--features",
            "codec-json,codec-lisp,device-reference,glasses-modeled,gpu-math,interference,music-algorithms,music-consonance,music-counterpoint,numbers-arith,numbers-f64,python,serial-music,stream-core",
        ])
        .status()
        .map_err(|err| format!("run cookbook recipe gate: {err}"))?;
    if !cookbook_status.success() {
        return Err(format!(
            "check-recipes cookbook gate failed with status {cookbook_status}"
        ));
    }

    let hotload_status = Command::new("cargo")
        .args([
            "test",
            "--test",
            "hotload_generation",
            "--no-default-features",
            "--features",
            "hotload",
        ])
        .status()
        .map_err(|err| format!("run public hotload recipe: {err}"))?;
    if !hotload_status.success() {
        return Err(format!(
            "check-recipes hotload gate failed with status {hotload_status}"
        ));
    }

    println!("check-recipes: OK (seeded cookbook and public hotload recipe gates passed)");
    Ok(())
}
