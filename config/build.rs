fn main() {
    // Read at build time, so `SLIPPI_ENV=production cargo build` picks the production settings.
    // Rebuild whenever it changes, or a switch would keep the old environment.
    println!("cargo:rerun-if-env-changed=SLIPPI_ENV");

    let env = std::env::var("SLIPPI_ENV").unwrap_or_else(|_| String::from("development"));
    println!("cargo:warning=Using Env {}", env);

    // A cfg rather than a Cargo feature, so `--all-features` can't enable two environments.
    println!("cargo::rustc-check-cfg=cfg(slippi_env, values(\"development\", \"staging\", \"production\"))");
    println!("cargo::rustc-cfg=slippi_env=\"{}\"", env.to_lowercase());
}
