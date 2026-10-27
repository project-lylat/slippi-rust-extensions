fn main() {
    println!("cargo:rerun-if-env-changed=SLIPPI_ENV");

    let env = std::env::var("SLIPPI_ENV").unwrap_or_else(|_| String::from("development"));
    println!("cargo:warning=Using Env {}", env);

    println!("cargo:rustc-cfg=feature=\"slippi_env_{}\"", env.to_lowercase());
}
