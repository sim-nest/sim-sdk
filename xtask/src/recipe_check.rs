use std::process::Command;

pub fn run(args: &[String]) -> Result<(), String> {
    let program = args.first().map(String::as_str).unwrap_or("xtask");
    if args.len() != 2 {
        return Err(format!("usage: {program} check-recipes"));
    }

    let status = Command::new("cargo")
        .args([
            "test",
            "--test",
            "cookbook",
            "cookbook_recipe_gate_runs_every_seeded_sdk_recipe",
            "--features",
            "codec-json,codec-lisp,device-reference,glasses-modeled,gpu-math,interference,music-algorithms,music-consonance,music-counterpoint,numbers-arith,numbers-f64,serial-music,stream-core",
        ])
        .status()
        .map_err(|err| format!("run cookbook recipe gate: {err}"))?;
    if status.success() {
        println!("check-recipes: OK (seeded cookbook recipe gate passed)");
        Ok(())
    } else {
        Err(format!("check-recipes failed with status {status}"))
    }
}
