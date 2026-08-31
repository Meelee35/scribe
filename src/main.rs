mod data;
mod app;

use crate::app::App;

fn main() -> anyhow::Result<()> {
    let app_data = data::load()?;
    ratatui::run(|terminal| App::new(app_data).run(terminal))?;

    Ok(())
}
