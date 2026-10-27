//! Original import sites and complete compiler-observed foreign export routes.
use crate::{
    generated_sources::capture::Definition, nominal_boundary::census::expansions::FileSpan,
};
use serde::{Deserialize, Serialize};

pub const KEY: &str = "rstyle.rust.dependency-namespaces";
pub const IDENTITY: &str = "rstyle.rust.dependency-namespace-capture/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitCapture {
    pub identity: String,
    pub imports: Vec<Import>,
    pub declarations: Vec<Declaration>,
    pub unsupported: Vec<Unsupported>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "name",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Binding {
    Glob,
    Anonymous,
    Named(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub definition: Definition,
    pub declaration: FileSpan,
    pub leaf: FileSpan,
    pub owner: Definition,
    pub scope: Vec<String>,
    pub path: Vec<String>,
    pub binding: Binding,
    pub public: bool,
    pub target: Definition,
    pub target_kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Import {
    pub site: Site,
    pub compiled_dependency: serde_json::Value,
    pub exports: Vec<Export>,
    pub complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub site: Site,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Export {
    pub binding: Binding,
    pub target: Definition,
    pub target_kind: String,
    pub public: bool,
    pub route: Vec<Route>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub kind: RouteKind,
    pub declaration: Definition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteKind {
    Single,
    Glob,
    ExternCrate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unsupported {
    pub definition: Definition,
    pub reason: String,
}
