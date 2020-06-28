use std::fmt::Debug;

use lazy_static::*;
use serde::Serialize;
use tera::Tera;

pub use action::*;
use include_dir::{include_dir, Dir};
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
    static ref TEMPLATES: Tera = {
        // Include all widget templates at compile-time:
        static TEMPLATE_DIR: Dir = include_dir!("src/res/widgets/");

        let mut tera = Tera::default();

        // Add 'base_*' templates first, because others depend on them.
        let base_static = TEMPLATE_DIR.get_file("base_static.j2").unwrap();
        let base_dynamic = TEMPLATE_DIR.get_file("base_dynamic.j2").unwrap();
        tera.add_raw_template("base_static.j2", base_static.contents_utf8().unwrap()).unwrap();
        tera.add_raw_template("base_dynamic.j2", base_dynamic.contents_utf8().unwrap()).unwrap();

        // Add all other templates.
        for file in TEMPLATE_DIR.files() {
            tera.add_raw_template(
                file.path().to_str().unwrap(),
                file.contents_utf8().unwrap()
            ).unwrap();
        }

        tera
    };
}
