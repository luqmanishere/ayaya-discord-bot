use anyhow::Result;
use vergen_gix::{BuildBuilder, CargoBuilder, Emitter, GixBuilder, RustcBuilder};

fn main() -> Result<()> {
    let build = BuildBuilder::all_build().unwrap();
    let cargo = CargoBuilder::all_cargo().unwrap();
    let gix = GixBuilder::default()
        .all()
        .describe(true, false, None)
        .build()
        .unwrap();
    let rustc = RustcBuilder::all_rustc().unwrap();

    Emitter::default()
        .fail_on_error()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&gix)?
        .add_instructions(&rustc)?
        .emit()?;

    Ok(())
}
