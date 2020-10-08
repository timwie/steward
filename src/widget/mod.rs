use std::fmt::{Debug, Display};

use askama::Template;

pub use action::*;
pub use common::*;

mod action;
mod common;
mod filters;
pub mod timeattack;

#[derive(Template, Debug)]
#[template(path = "common/root.xml")]
pub struct Manialink<'a, T>
where
    T: Template,
    T: Debug,
    T: Display,
{
    pub id: &'a str,
    pub name: &'a str,
    pub widget: &'a T,
}

#[derive(Template, Debug)]
#[template(source = "", ext = "xml")]
pub struct EmptyWidget {}
