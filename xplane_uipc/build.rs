use std::{env, path::PathBuf};

use vergen_git2::{BuildBuilder, Emitter, Git2Builder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // println!("cargo:warning=Building now");
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Failed to get CARGO_MANIFEST_DIR");
    let root = PathBuf::from(manifest_dir);

    let sdk_root = root
        .join("vendor")
        .join("x-plane-sdk")
        .join("4.3.0")
        .join("SDK");
    let win_libs = sdk_root.join("Libraries").join("Win");
    let xplm_c_headers = sdk_root.join("CHeaders").join("XPLM");
    let xpwidgets_c_headers = sdk_root.join("CHeaders").join("Widgets");

    // Tell cargo where the .lib files are
    println!("cargo:rustc-link-search=native={}", win_libs.display());

    // Link the X-Plane libraries
    println!("cargo:rustc-link-lib=XPLM_64");
    println!("cargo:rustc-link-lib=XPWidgets_64");

    let bindings = bindgen::Builder::default()
        .wrap_unsafe_ops(true)
        .header("xplane_sdk.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_arg(format!("-I{}", xplm_c_headers.display()))
        .clang_arg(format!("-I{}", xpwidgets_c_headers.display()))
        // allow-list functions
        .allowlist_function("XPLMDebugString")
        .allowlist_function("XPLMFindPluginsMenu")
        .allowlist_function("XPLMAppendMenuItem")
        .allowlist_function("XPLMCreateMenu")
        .allowlist_function("XPLMSetGraphicsState")
        .allowlist_function("XPLMCreateWindowEx")
        .allowlist_function("XPLMSetWindowTitle")
        .allowlist_function("XPLMDrawString")
        .allowlist_function("XPLMGetWindowGeometry")
        .allowlist_function("XPIsWidgetVisible")
        .allowlist_function("XPHideWidget")
        .allowlist_function("XPShowWidget")
        .allowlist_function("XPCreateWidget")
        .allowlist_function("XPSetWidgetProperty")
        .allowlist_function("XPAddWidgetCallback")
        .allowlist_function("XPLMEnableFeature")
        .allowlist_function("XPLMGetSystemPath")
        .allowlist_function("XPLMGetPrefsPath")
        .allowlist_function("XPLMFindDataRef")
        .allowlist_function("XPLMGetDataRefTypes")
        .allowlist_function("XPLMGetDatad")
        .allowlist_function("XPLMGetDataf")
        .allowlist_function("XPLMGetDatai")
        .allowlist_function("XPLMGetDatavf")
        .allowlist_function("XPLMGetDatavi")
        .allowlist_function("XPLMSetDatad")
        .allowlist_function("XPLMSetDataf")
        .allowlist_function("XPLMSetDatai")
        .allowlist_function("XPLMRegisterFlightLoopCallback")
        .allowlist_function("XPLMUnregisterFlightLoopCallback")
        .allowlist_function("XPLMCreateFlightLoop")
        .allowlist_function("XPLMSetFlightLoopCallbackInterval")
        // allow-list types
        .allowlist_type("XPLMMenuID")
        .allowlist_type("XPWidgetMessage")
        .allowlist_type("XPWidgetID")
        .allowlist_type("XPLMDataRef")
        // allow-list variables
        .allowlist_var("xplmFont_Proportional")
        .allowlist_var("xpWidgetClass_MainWindow")
        .allowlist_var("xpProperty_MainWindowHasCloseBoxes")
        .allowlist_var("xpMainWindowStyle_Translucent")
        .allowlist_var("xpWidgetClass_Caption")
        .allowlist_var("xpProperty_CaptionLit")
        .allowlist_var("xpMessage_CloseButtonPushed")
        .allowlist_var("XPLM_USE_NATIVE_PATHS")
        .allowlist_var("XPLM_USE_NATIVE_WIDGET_WINDOWS")
        .default_macro_constant_type(bindgen::MacroTypeVariation::Signed)
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").expect("Failed to get OUT_DIR"));
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // capture build & version information
    let build = BuildBuilder::all_build()?;
    let git2 = Git2Builder::all_git()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&git2)?
        .emit()?;
    Ok(())
}
