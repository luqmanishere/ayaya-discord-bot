use anyhow::Result;
use vergen_gix::{Build, Cargo, Emitter, Gix, Rustc};

fn main() -> Result<()> {
    let build = Build::all_build();
    let cargo = Cargo::all_cargo();
    let gix = Gix::all().describe(true, false, None).build();
    let rustc = Rustc::all_rustc();

    Emitter::default()
        .fail_on_error()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&gix)?
        .add_instructions(&rustc)?
        .emit()?;

    Ok(())
}
