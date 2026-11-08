//! Which analyses a run performs: core's own, and those local to one frontend.

use crate::cpg::records::Demand;
use std::collections::BTreeSet;

/// An analysis core runs over a run's documents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Analysis {
    Check,
    Patterns,
    Bounds,
    Flow,
    Defects,
}

impl Analysis {
    const ALL: [Self; 5] = [
        Self::Check,
        Self::Patterns,
        Self::Bounds,
        Self::Flow,
        Self::Defects,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Patterns => "patterns",
            Self::Bounds => "bounds",
            Self::Flow => "flow",
            Self::Defects => "defects",
        }
    }

    fn named(name: &str) -> Option<Self> {
        match name {
            "check" => Some(Self::Check),
            "patterns" => Some(Self::Patterns),
            "bounds" => Some(Self::Bounds),
            "flow" => Some(Self::Flow),
            "defects" => Some(Self::Defects),
            _ => None,
        }
    }
}

/// An analysis one frontend runs itself, written `<frontend>:<name>`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrontendAnalysis {
    pub frontend: String,
    pub name: String,
}

impl FrontendAnalysis {
    fn belongs_to(&self, language: &str) -> Result<(), String> {
        match self.frontend == language {
            true => Ok(()),
            false => Err(format!(
                "`{self}` belongs to the {} frontend, not {language}",
                self.frontend
            )),
        }
    }
}

impl std::fmt::Display for FrontendAnalysis {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{}", self.frontend, self.name)
    }
}

/// What a run analyzes. Without a filter, that is everything.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    Everything,
    Only {
        core: BTreeSet<Analysis>,
        frontend: BTreeSet<FrontendAnalysis>,
    },
}

impl Filter {
    /// Selects the named analyses; no names at all means everything.
    pub fn parse<S: AsRef<str>>(names: &[S]) -> Result<Self, String> {
        if names.is_empty() {
            return Ok(Self::Everything);
        }
        let mut core = BTreeSet::new();
        let mut frontend = BTreeSet::new();

        for name in names {
            match parsed(name.as_ref())? {
                Selected::Core(analysis) => core.insert(analysis),
                Selected::Frontend(analysis) => frontend.insert(analysis),
            };
        }
        Ok(Self::Only { core, frontend })
    }

    pub fn core(&self, analysis: Analysis) -> bool {
        match self {
            Self::Everything => true,
            Self::Only { core, .. } => core.contains(&analysis),
        }
    }

    /// Whether anything needs a run's documents. A run of only frontend analyses doesn't.
    pub fn needs_documents(&self) -> bool {
        Analysis::ALL
            .into_iter()
            .any(|analysis| self.core(analysis))
    }

    /// What the selected core analyses ask of each document.
    pub fn demand(&self) -> Demand {
        Demand {
            graphs: self.core(Analysis::Patterns),
            bounds: self.core(Analysis::Bounds),
            flow: self.core(Analysis::Flow),
            defects: self.core(Analysis::Defects),
        }
    }

    /// The frontend analyses named explicitly, which nothing can run without a frontend.
    pub fn named_frontend_analyses(&self) -> Vec<&FrontendAnalysis> {
        match self {
            Self::Everything => Vec::new(),
            Self::Only { frontend, .. } => frontend.iter().collect(),
        }
    }

    /// Fails on an analysis of another frontend. It takes no frontend to tell, so a run can
    /// refuse before looking for one, which may install it.
    pub fn names_only_frontend(&self, language: &str) -> Result<(), String> {
        self.named_frontend_analyses()
            .into_iter()
            .try_for_each(|analysis| analysis.belongs_to(language))
    }

    /// The analyses `language`'s frontend should run, given the ones it offers. A name it
    /// doesn't offer, or one for another frontend, fails before anything runs.
    pub fn frontend_analyses(
        &self,
        language: &str,
        available: &[String],
    ) -> Result<Vec<String>, String> {
        let Self::Only { frontend, .. } = self else {
            return Ok(available.to_vec());
        };
        frontend
            .iter()
            .map(|analysis| offered(analysis, language, available))
            .collect()
    }
}

/// Names become file names, so a frontend's name can never climb out of its folder.
pub fn valid_name(name: &str) -> bool {
    let allowed = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-';
    name.as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && name.bytes().all(allowed)
}

enum Selected {
    Core(Analysis),
    Frontend(FrontendAnalysis),
}

fn parsed(name: &str) -> Result<Selected, String> {
    if let Some((frontend, analysis)) = name.split_once(':') {
        return frontend_analysis(name, frontend, analysis).map(Selected::Frontend);
    }
    Analysis::named(name).map(Selected::Core).ok_or_else(|| {
        format!(
            "Unknown analysis `{name}`: expected check, patterns, bounds, flow, defects or <frontend>:<analysis>"
        )
    })
}

fn frontend_analysis(
    written: &str,
    frontend: &str,
    name: &str,
) -> Result<FrontendAnalysis, String> {
    if !(valid_name(frontend) && valid_name(name)) {
        return Err(format!(
            "Invalid analysis `{written}`: use letters, digits, `_` and `-` on both sides of `:`"
        ));
    }
    Ok(FrontendAnalysis {
        frontend: frontend.into(),
        name: name.into(),
    })
}

fn offered(
    analysis: &FrontendAnalysis,
    language: &str,
    available: &[String],
) -> Result<String, String> {
    analysis.belongs_to(language)?;
    if available.contains(&analysis.name) {
        return Ok(analysis.name.clone());
    }
    Err(match available {
        [] => format!(
            "The {language} frontend has no analysis `{}`: it offers none",
            analysis.name
        ),
        names => format!(
            "The {language} frontend has no analysis `{}`: it offers {}",
            analysis.name,
            names.join(", ")
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn only(names: &[&str]) -> Filter {
        Filter::parse(names).unwrap()
    }

    #[test]
    fn no_filter_selects_everything_and_what_it_needs() {
        let filter = only(&[]);
        assert_eq!(filter, Filter::Everything);
        assert!(filter.core(Analysis::Check));
        assert!(filter.core(Analysis::Patterns));
        assert!(filter.core(Analysis::Bounds));
        assert!(filter.core(Analysis::Flow));
        assert!(filter.core(Analysis::Defects));
        assert!(filter.needs_documents());
        let demand = filter.demand();
        assert!(demand.graphs && demand.bounds && demand.flow && demand.defects);
    }

    #[test]
    fn a_filter_selects_only_its_names() {
        let filter = only(&["check", "bounds", "check"]);
        assert!(filter.core(Analysis::Check));
        assert!(!filter.core(Analysis::Patterns));
        assert!(filter.core(Analysis::Bounds));
        assert!(!filter.core(Analysis::Flow));
        let demand = filter.demand();
        assert!(!demand.graphs && demand.bounds && !demand.flow && !demand.defects);
        assert!(only(&["flow"]).demand().flow);
        assert!(only(&["defects"]).demand().defects);
    }

    #[test]
    fn only_core_analyses_need_documents() {
        assert!(!only(&["go:calls"]).needs_documents());
        assert!(only(&["go:calls", "patterns"]).needs_documents());
    }

    #[test]
    fn unknown_or_unsafe_names_are_refused() {
        for name in [
            "clones", "", "go:", ":calls", "go:../x", "go:a/b", "../go:a", "go:a:b", "go:-a",
        ] {
            assert!(Filter::parse(&[name]).is_err(), "{name}");
        }
    }

    #[test]
    fn without_a_filter_a_frontend_runs_all_it_offers() {
        let offered = vec!["calls".to_string(), "taint".to_string()];
        assert_eq!(
            only(&[]).frontend_analyses("go", &offered).unwrap(),
            offered
        );
        assert!(only(&[]).named_frontend_analyses().is_empty());
    }

    #[test]
    fn a_filter_runs_only_what_it_names_of_a_frontend() {
        let offered = vec!["calls".to_string(), "taint".to_string()];
        let filter = only(&["go:taint", "check"]);
        assert_eq!(filter.frontend_analyses("go", &offered).unwrap(), ["taint"]);
        assert!(
            only(&["check"])
                .frontend_analyses("go", &offered)
                .unwrap()
                .is_empty()
        );
        assert_eq!(filter.named_frontend_analyses()[0].to_string(), "go:taint");
    }

    #[test]
    fn another_frontends_analysis_fails_with_nothing_on_offer_to_ask() {
        assert_eq!(Filter::Everything.names_only_frontend("go"), Ok(()));
        assert_eq!(only(&["go:calls"]).names_only_frontend("go"), Ok(()));
        let other = only(&["go:calls", "rust:calls"])
            .names_only_frontend("go")
            .unwrap_err();
        assert!(other.contains("belongs to the rust frontend"), "{other}");
    }

    #[test]
    fn a_name_the_frontend_lacks_fails_before_anything_runs() {
        let offered = vec!["calls".to_string()];
        let other = only(&["rust:calls"])
            .frontend_analyses("go", &offered)
            .unwrap_err();
        assert!(other.contains("belongs to the rust frontend"), "{other}");
        let typo = only(&["go:call"])
            .frontend_analyses("go", &offered)
            .unwrap_err();
        assert!(typo.contains("it offers calls"), "{typo}");
        let none = only(&["go:call"]).frontend_analyses("go", &[]).unwrap_err();
        assert!(none.contains("it offers none"), "{none}");
    }
}
