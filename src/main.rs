use anyhow::{bail, Context, Result};
use clap::clap_app;
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Serialize)]
struct CargoConfig {
  package: CargoPackage,
  lib: CargoLib,
  dependencies: HashMap<String, CargoDependency>,
}

#[derive(Serialize)]
struct CargoPackage {
  name: String,
  version: String,
  edition: String,
}

#[derive(Serialize)]
struct CargoLib {
  name: String,
  #[serde(rename = "crate-type")]
  crate_type: Vec<String>,
}

#[derive(Serialize)]
struct CargoDependency {
  version: String,
  features: Vec<String>,
  git: Option<String>,
  branch: Option<String>,
}

fn collect_deps(input: &Path) -> Result<Vec<String>> {
  let src = String::from_utf8(fs::read(input)?)?;
  let mut deps = Vec::new();
  for line in src.lines() {
    if !line.starts_with("// ") {
      break;
    }

    let dep = line.chars().skip(3).collect::<String>();
    deps.push(dep)
  }

  Ok(deps)
}

fn create_dir(
  cargo_dir: &Path,
  input: &Path,
  crate_name: &str,
  module_name: &str,
  deps: &[String],
  pyo3_version: &str,
) -> Result<()> {
  let mut dependencies = HashMap::new();
  let (version, git, branch) = if pyo3_version == "github" {
    (
      "*".into(),
      Some("https://github.com/PyO3/pyo3".into()),
      Some("main".into()),
    )
  } else {
    (pyo3_version.into(), None, None)
  };

  dependencies.insert(
    "pyo3".into(),
    CargoDependency {
      features: vec!["extension-module".into()],
      version,
      git,
      branch,
    },
  );

  let config = CargoConfig {
    package: CargoPackage {
      name: crate_name.into(),
      version: "0.1.0".into(),
      edition: "2018".into(),
    },
    lib: CargoLib {
      name: module_name.into(),
      crate_type: vec!["cdylib".into()],
    },
    dependencies,
  };

  let src_dir = &cargo_dir.join("src");
  fs::create_dir_all(src_dir)?;

  let mut config_contents = toml::to_string(&config)?;
  config_contents.push_str(&format!("\n[dependencies]\n{}", deps.join("\n")));

  fs::write(cargo_dir.join("Cargo.toml"), config_contents)?;
  fs::copy(input, src_dir.join("lib.rs"))?;

  let dot_cargo = cargo_dir.join(".cargo");
  fs::create_dir_all(&dot_cargo)?;
  fs::write(
    dot_cargo.join("config.toml"),
    r#"
[target.x86_64-apple-darwin]
rustflags = [
  "-C", "link-arg=-undefined",
  "-C", "link-arg=dynamic_lookup",
]

[target.aarch64-apple-darwin]
rustflags = [
  "-C", "link-arg=-undefined",
  "-C", "link-arg=dynamic_lookup",
]"#,
  )?;

  Ok(())
}

fn run() -> Result<()> {
  let clap_args = env::args().skip(1).collect::<Vec<_>>();
  let matches = clap_app! {single_pyo3 =>
    (version: "0.1")
    (author: "Will Crichton <crichton.will@gmail.com>")
    (about: "Builds a single Rust file as a Python module via pyo3")
    (@arg verbose: -v --verbose)
    (@arg release: --release)
    (@arg pyo3: --pyo3 +takes_value "Pyo3 version. Use \"github\" to get latest from main branch.")
    (@arg INPUT: +required "Input file")
  }
  .get_matches_from(&clap_args);

  let verbose = matches.is_present("verbose");
  let input = matches.value_of("INPUT").unwrap();
  let input = Path::new(input);

  let crate_name = input
    .file_stem()
    .context("No file stem")?
    .to_str()
    .context("to_string")?;
  let module_name = crate_name.replace('-', "_");

  let cargo_dir = &env::temp_dir().join(crate_name);
  if verbose {
    println!("{}", cargo_dir.display());
  }

  let deps = collect_deps(input)?;

  create_dir(
    cargo_dir,
    input,
    crate_name,
    &module_name,
    &deps,
    matches.value_of("pyo3").unwrap_or("*"),
  )?;

  let is_release = matches.is_present("release");
  let mut args = vec!["build"];
  if is_release {
    args.push("--release");
  }
  let status = Command::new("cargo")
    .args(&args)
    .current_dir(cargo_dir)
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .status()?;

  if !status.success() {
    bail!("cargo failed");
  }

  let lib_name = format!("lib{}.{}", module_name, env::consts::DLL_EXTENSION);
  let release = if is_release { "release" } else { "debug" };
  let lib_src_path = cargo_dir.join("target").join(release).join(lib_name);
  let lib_dst_path = format!("{}.so", module_name);
  fs::copy(lib_src_path, lib_dst_path)?;

  Ok(())
}

fn main() {
  run().unwrap();
}
