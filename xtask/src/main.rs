use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use dircpy::CopyBuilder;

type DynError = Box<dyn std::error::Error>;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("dist") => dist()?,
        Some("deploy") => deploy()?,
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:

dist            builds application and copies assets to dist/ for packaging
deploy          builds and deploys to X-Plane plugins directory
"
    )
}

fn deploy() -> Result<(), DynError> {
    dist()?;

    let dist_dir = dist_dir();
    let dest_dir = PathBuf::from(r"C:\X-Plane 12\Resources\plugins\xplane-uipc");
    CopyBuilder::new(&dist_dir, &dest_dir)
        .overwrite(true)
        .with_progress(|all, done| {
            println!("copied {done}/{all}");
        })
        .run()
        .unwrap();

    Ok(())
}

fn dist() -> Result<(), DynError> {
    let _ = fs::remove_dir_all(&dist_dir());
    fs::create_dir_all(&dist_dir())?;

    dist_binary()?;
    // dist_manpage()?;

    Ok(())
}

fn dist_binary() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .args(&["build", "--release"])
        .status()?;

    if !status.success() {
        Err("cargo build failed")?;
    }

    eprintln!("cargo build succeeded");

    let src = project_root()
        .join("target")
        .join("release")
        .join("xplane_uipc.dll");
    let dll_dir = dist_dir().join("win_x64");
    fs::create_dir_all(&dll_dir)?;
    eprintln!("copying {} to {}", src.display(), dll_dir.display());
    fs::copy(&src, dll_dir.join("xplane-uipc.xpl"))?;

    let src_license = project_root().join("LICENSE.md");
    fs::copy(&src_license, dist_dir().join("LICENSE.md"))?;
    let src_readme = project_root().join("README.md");
    fs::copy(&src_readme, dist_dir().join("README.md"))?;
    let src_mappings = project_root().join("xplane_uipc").join("mappings.toml");
    fs::copy(&src_mappings, dist_dir().join("mappings.toml"))?;
    let src_config = project_root().join("xplane_uipc").join("config.toml");
    fs::copy(&src_config, dist_dir().join("config.toml"))?;

    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}

fn dist_dir() -> PathBuf {
    project_root().join("dist").join("xplane-uipc")
}
