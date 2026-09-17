use std::{env, path::PathBuf, process::Command};
fn run(command: &mut Command) {
    let status = command.status().expect("could not launch build tool (install Flex and a C toolchain)");
    assert!(status.success(), "build tool failed: {command:?}");
}
fn main() {
    for file in ["lexer/netlang.l", "lexer/lexer_bridge.c", "lexer/lexer_bridge.h"] {
        println!("cargo:rerun-if-changed={file}");
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    run(Command::new("flex").arg("-o").arg(out.join("lex.yy.c")).arg("lexer/netlang.l"));
    for (source, name) in [(out.join("lex.yy.c"), "scanner"), (PathBuf::from("lexer/lexer_bridge.c"), "bridge")] {
        run(Command::new(env::var("CC").unwrap_or("cc".into())).arg("-std=c99").arg("-D_POSIX_C_SOURCE=200809L").arg("-Ilexer").arg("-c").arg(source).arg("-o").arg(out.join(format!("{name}.o"))));
    }
    run(Command::new(env::var("AR").unwrap_or("ar".into())).arg("crs").arg(out.join("libnetlang_lexer.a")).arg(out.join("scanner.o")).arg(out.join("bridge.o")));
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=netlang_lexer");
}
