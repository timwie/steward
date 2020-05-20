pub(self) use chat::*;
pub use facade::Controller;
pub(self) use player::*;
pub(self) use playlist::*;
pub(self) use preference::*;
pub use preference::{ActivePreference, ActivePreferenceValue};
pub(self) use queue::*;
pub(self) use race::*;
pub(self) use record::*;
pub(self) use server_rank::*;
pub(self) use settings::*;
pub(self) use widget::*;

mod chat;
mod compat;
mod facade;
mod player;
mod playlist;
mod preference;
mod queue;
mod race;
mod record;
mod server_rank;
mod settings;
mod widget;
