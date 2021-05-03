use anyhow::{bail, Context, Result};
use clap::clap_app;
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

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
  use_github: bool,
) -> Result<()> {
  let mut dependencies = HashMap::new();
  let (git, branch) = if use_github {
    (
      Some("https://github.com/PyO3/pyo3".into()),
      Some("main".into()),
    )
  } else {
    (None, None)
  };

  dependencies.insert(
    "pyo3".into(),
    CargoDependency {
      version: "0.13".into(),
      features: vec!["extension-module".into()],
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

  Ok(())
}

fn run() -> Result<()> {
  let matches = clap_app! {single_pyo3 =>
    (version: "0.1")
    (author: "Will Crichton <crichton.will@gmail.com>")
    (about: "Builds a single Rust file as a Python module via pyo3")
    (@arg verbose: -v --verbose)
    (@arg github: --github)
    (@arg release: --release)
    (@arg INPUT: +required "Input file")
  }
  .get_matches();

  let verbose = matches.is_present("verbose");
  let input = matches.value_of("INPUT").unwrap();
  let input = Path::new(input);

  let crate_name = input
    .file_stem()
    .context("No file stem")?
    .to_str()
    .context("to_string")?;
  let module_name = crate_name.replace("-", "_");

  let cargo_dir = &env::temp_dir().join(crate_name);
  if verbose {
    println!("{}", cargo_dir.display());
  }

  let deps = collect_deps(input)?;

  create_dir(
    &cargo_dir,
    input,
    &crate_name,
    &module_name,
    &deps,
    matches.is_present("github"),
  )?;

  let is_release = matches.is_present("release");
  let mut args = vec!["build"];
  if is_release {
    args.push("--release");
  }
  let output = Command::new("cargo")
    .args(&args)
    .current_dir(cargo_dir)
    .output()?;

  if !output.status.success() {
    bail!("{}", String::from_utf8(output.stderr)?);
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
