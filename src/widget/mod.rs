use std::fmt::Debug;

use include_dir::{Dir, File};
use lazy_static::*;
use serde::Serialize;
use tera::Tera;

pub use action::*;
pub use intro::*;
pub use outro_map_rankings::*;
pub use outro_queue::*;
pub use outro_server_ranking::*;
pub use popup::*;
pub use race_live_ranks::*;
pub use race_run_outro::*;
pub use race_sector_diff::*;
pub use race_toggle_menu::*;

use crate::config::{CDN_PREFIX, CDN_PREFIX_MASTER};

mod action;
mod intro;
mod outro_map_rankings;
mod outro_queue;
mod outro_server_ranking;
mod popup;
mod race_live_ranks;
mod race_run_outro;
mod race_sector_diff;
mod race_toggle_menu;
mod ser;

pub trait Widget
where
    Self: Serialize + Sized + Debug,
{
    /// Must be a file name ending in `.j2`, located in `src/res/widgets/`.
    const FILE: &'static str;

    /// Manialink ID for this widget. Defaults to its file name.
    ///
    /// Choosing the same ID for multiple widgets allows replacing
    /// one widget with another.
    const ID: &'static str = Self::FILE;

    /// Render the template file with this context, extended by
    /// - `widget_id`: use as `<manialink>` ID
    /// - `cdn`: prefix URL for images in `src/res/img`
    fn render(&self) -> String {
        log::debug!("render widget context: {:?}", &self);

        let mut tera_context =
            tera::Context::from_serialize(self).expect("failed to create widget context!");
        Self::extend_ctxt(&mut tera_context);

        TEMPLATES
            .render(Self::FILE, &tera_context)
            .expect("failed to render widget!")
    }

    /// Render an empty widget that can replace a previously sent widget
    /// of this type.
    fn hidden() -> String {
        let mut tera_context = tera::Context::new();
        Self::extend_ctxt(&mut tera_context);
        TEMPLATES
            .render("empty.j2", &tera_context)
            .expect("failed to render widget!")
    }

    fn extend_ctxt(ctxt: &mut tera::Context) {
        ctxt.insert("widget_id", Self::ID);
        ctxt.insert(
            "cdn",
            if cfg!(debug_assertions) {
                CDN_PREFIX_MASTER
            } else {
                CDN_PREFIX
            },
        );
    }
}

lazy_static! {
    static ref TEMPLATES: Tera = collect_templates().unwrap();
}

fn collect_templates() -> tera::Result<Tera> {
    // Include all widget templates at compile-time:
    static TEMPLATE_DIR: Dir = include_dir!("src/res/widgets/");

    let mut tera = Tera::default();

    let add_from_file = |tera: &mut Tera, file: &File| {
        let file_name = file.path().to_str().expect("failed to read template");
        tera.add_raw_template(
            file_name,
            file.contents_utf8().expect("failed to read template"),
        )
    };

    let add_from_name = |tera: &mut Tera, file_name: &str| {
        let file = TEMPLATE_DIR
            .get_file(file_name)
            .expect("failed to find template");
        add_from_file(tera, &file)
    };

    // Add 'base_*' templates first, because others depend on them.
    add_from_name(&mut tera, "base_static.j2")?;
    add_from_name(&mut tera, "base_dynamic.j2")?;

    // Add all other templates.
    for file in TEMPLATE_DIR.files() {
        add_from_file(&mut tera, file)?;
    }

    Ok(tera)
}
