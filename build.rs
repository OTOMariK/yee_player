#[cfg(windows)]
use winres::WindowsResource;

fn main() -> std::io::Result<()> {
    #[cfg(windows)]
    {
        WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("./asset/icon.ico")
            .compile()?;
    }
    Ok(())
}
