fn main() {
  println!("cargo::rustc-check-cfg=cfg(desktop)");

  let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
  let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

  if target_arch != "wasm32" && target_os != "android" {
    println!("cargo:rustc-cfg=desktop");
  }

  println!("cargo:rerun-if-changed=build.rs");
}
