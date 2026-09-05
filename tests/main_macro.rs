use std::{
    fs,
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

struct Application {
    directory: PathBuf,
    target: PathBuf,
}

impl Application {
    fn new() -> Self {
        let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/main-macro-tests");
        let directory = target.join(format!("application-{}", std::process::id()));
        fs::create_dir_all(directory.join("src")).unwrap();
        let application = Self { directory, target };
        application.dependency("lunacy");
        application
    }

    fn dependency(&self, name: &str) {
        fs::write(
            self.directory.join("Cargo.toml"),
            format!(
                r#"[package]
name = "lunacy-main-probe"
version = "0.0.0"
edition = "2024"

[workspace]

[dependencies]
{name} = {{ package = "lunacy", path = {path:?}, default-features = false, features = ["macros"] }}

[profile.dev]
panic = "abort"
"#,
                path = env!("CARGO_MANIFEST_DIR"),
            ),
        )
        .unwrap();
    }

    fn compile(&self, source: &str, command: &str) -> Output {
        fs::write(
            self.directory.join("src/main.rs"),
            format!("#![no_std]\n#![no_main]\n{source}"),
        )
        .unwrap();
        Command::new(env!("CARGO"))
            .args([command, "--offline", "--manifest-path"])
            .arg(self.directory.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", &self.target)
            .output()
            .unwrap()
    }

    fn build(&self, source: &str) {
        let output = self.compile(source, "build");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn executable(&self) -> PathBuf {
        self.target.join("debug/lunacy-main-probe")
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn attribute_supports_downstream_no_std_programs_and_reports_invalid_use() {
    let application = Application::new();
    application.build(
        r#"
extern crate alloc;
use alloc::boxed::Box;

#[lunacy::main]
fn main(args: lunacy::Args<'_>) -> i32 {
    let text = args.get(1).unwrap();
    let code = Box::new(37);
    assert_eq!(args.len(), 2);
    assert_eq!(lunacy::println!("{}", text.to_str().unwrap()), Ok(6));
    *code
}
"#,
    );
    let output = Command::new(application.executable())
        .arg("hello")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(37));
    assert_eq!(output.stdout, b"hello\n");

    // Both attribute orders and nested cfg_attr must disable runtime setup
    // alongside the function; function-only attributes must not reach it.
    application.build(
        r#"
#[lunacy::main]
#[cfg(any())]
fn absent(_: lunacy::Args<'_>) -> i32 { 1 }

#[cfg(any())]
#[lunacy::main]
fn also_absent(_: lunacy::Args<'_>) -> i32 { 2 }

#[lunacy::main]
#[cfg_attr(all(), inline, cfg_attr(all(), cfg(any())))]
fn conditional(_: lunacy::Args<'_>) -> i32 { 3 }

mod program {
    #[lunacy::main]
    #[cfg_attr(all(), inline)]
    pub(super) fn r#start(args: lunacy::Args<'_>) -> i32 {
        let _: fn(lunacy::Args<'_>) -> i32 = super::program::r#start;
        args.len() as i32
    }
}
"#,
    );
    let status = Command::new(application.executable()).status().unwrap();
    assert_eq!(status.code(), Some(1));

    for (source, expected) in [
        (
            "#[lunacy::main] async fn main(_: lunacy::Args<'_>) -> i32 { 0 }",
            "safe, synchronous, non-generic",
        ),
        (
            "#[lunacy::main] unsafe fn main(_: lunacy::Args<'_>) -> i32 { 0 }",
            "safe, synchronous, non-generic",
        ),
        (
            "#[lunacy::main] fn main<T>(_: lunacy::Args<'_>) -> i32 { 0 }",
            "safe, synchronous, non-generic",
        ),
        (
            "#[lunacy::main] fn main() -> i32 { 0 }",
            "requires one Args<'_> argument",
        ),
        (
            "#[lunacy::main] fn main(_: u32) -> i32 { 0 }",
            "mismatched types",
        ),
        (
            "#[lunacy::main] fn main(_: lunacy::Args<'_>) {}",
            "mismatched types",
        ),
        ("#[lunacy::main] struct Main;", "expected `fn`"),
        (
            "#[lunacy::main(unknown)] fn main(_: lunacy::Args<'_>) -> i32 { 0 }",
            "expected `crate = path`",
        ),
    ] {
        let output = application.compile(source, "check");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "unexpected success: {source}");
        assert!(stderr.contains(expected), "{source}\n{stderr}");
    }

    application.dependency("luna");
    application.build(
        r#"
#[luna::main(crate = luna)]
fn entry(_: luna::Args<'_>) -> i32 { 23 }
"#,
    );
    assert_eq!(
        Command::new(application.executable())
            .status()
            .unwrap()
            .code(),
        Some(23)
    );

    application.build(
        r#"
#[luna::main(crate = luna)]
fn entry(_: luna::Args<'_>) -> i32 { panic!("abort probe") }
"#,
    );
    // Disable core dumps in this child only, as in the pthread panic test.
    let status = Command::new("sh")
        .args(["-c", "ulimit -c 0; exec \"$1\"", "lunacy-main-test"])
        .arg(application.executable())
        .output()
        .unwrap()
        .status;
    assert_eq!(status.signal(), Some(libc::SIGABRT));
}
