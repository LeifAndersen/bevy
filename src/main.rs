#[cfg(feature = "bevy_editor")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(bevy_internal::editor::cli()?)
}