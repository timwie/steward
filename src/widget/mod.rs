use std::fmt::Debug;

use include_dir::{include_dir, Dir, File};
use lazy_static::*;
use serde::Serialize;
use tera::Tera;

pub use action::*;
pub use menu::*;
pub use menu_map_ranking::*;
pub use menu_playlist::*;
pub use menu_schedule::*;
pub use menu_server_ranking::*;
pub use outro::*;
pub use outro_queue::*;
pub use outro_server_ranking::*;
pub use popup::*;
pub use race_live_ranks::*;
pub use race_run_outro::*;

use crate::constants::cdn_prefix;

mod action;
mod formatters;
mod menu;
mod menu_map_ranking;
mod menu_playlist;
mod menu_schedule;
mod menu_server_ranking;
mod outro;
mod outro_queue;
mod outro_server_ranking;
mod popup;
mod race_live_ranks;
mod race_run_outro;

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
    const ID: &'static str;

    /// Render the template file with this context, extended by
    /// - `widget_id`: use as `<manialink>` ID
    /// - `cdn`: prefix URL for images in `src/res/img`
    fn render(&self) -> String {
        log::debug!("render widget context: {:?}", &self);

        let mut tera_context =
            tera::Context::from_serialize(self).expect("failed to create widget context!");
        Self::add_constants(&mut tera_context);

        TEMPLATES
            .render(Self::FILE, &tera_context)
            .expect("failed to render widget")
    }

    /// Render an empty widget that can replace a previously sent widget
    /// of this type.
    fn hidden() -> String {
        let mut tera_context = tera::Context::new();
        Self::add_constants(&mut tera_context);
        TEMPLATES
            .render("empty.j2", &tera_context)
            .expect("failed to render widget")
    }

    fn add_constants(ctxt: &mut tera::Context) {
        ctxt.insert("widget_id", &Self::ID);
        ctxt.insert("cdn", &cdn_prefix());
    }
}

lazy_static! {
    static ref TEMPLATES: Tera = collect_templates().expect("failed to collect widget templates");
}

fn collect_templates() -> tera::Result<Tera> {
    // Include all widget templates at compile-time
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
    add_from_name(&mut tera, "base_menu.j2")?;

    // Add all other templates.
    for file in TEMPLATE_DIR.files() {
        add_from_file(&mut tera, file)?;
    }

    Ok(tera)
}

macro_rules! impl_widget {
    ($file:expr, $typ:ty) => {
        impl Widget for $typ {
            const FILE: &'static str = $file;
            const ID: &'static str = Self::FILE;
        }
    };
}

impl_widget!("menu.j2", MenuWidget);
impl_widget!("menu_map_ranking.j2", MapRankingWidget<'_>);
impl_widget!("menu_playlist.j2", PlaylistWidget<'_>);
impl_widget!("menu_schedule.j2", ScheduleWidget);
impl_widget!("menu_server_ranking.j2", ServerRankingWidget<'_>);
impl_widget!("outro.j2", OutroWidget<'_>);
impl_widget!("outro_queue.j2", OutroQueueWidget<'_>);
impl_widget!("outro_server_ranking.j2", OutroServerRankingWidget<'_>);
impl_widget!("popup.j2", PopupWidget<'_>);
impl_widget!("race_live_ranks.j2", LiveRanksWidget);
impl_widget!("race_run_outro.j2", RunOutroWidget);
