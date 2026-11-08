//! Commands built a call at a time, as Rust's `Command` is: which of those calls put a value where
//! a shell runs it as code. Each call is made on the command, so a function's own flows say what
//! it's made on: the calls whose result reaches its receiver.
//!
//! Only a command made and built in one function is read, by calls whose argument count is known.

use super::super::value_flow::{End, FlowCall, Start, ValueFlows};
use super::{Models, Reached, Risk, Shell, Sink, names, shell_code_at};
use std::collections::{BTreeMap, BTreeSet};

/// Each argument a function's calls put where a shell runs it as code, and what that risks.
pub(super) type ShellCode = BTreeMap<Reached, Risk>;

/// Which calls each call is made on.
type MadeOn = BTreeMap<usize, BTreeSet<usize>>;

/// What a call on a command adds is what it's given after the receiver.
const ADDED: usize = 1;

/// What a call does to a command it's made on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// Makes one: it's a call the sink names.
    Makes,
    /// Adds what it's given.
    AddsOne,
    /// Adds each element of a list written out for it.
    AddsEach,
    /// Gives it back as it was, as a call with no body here is read to.
    Passes,
    /// Could do anything to it, as a call with a body here could.
    Unknown,
}

impl Role {
    fn of(sink: &Sink, call: &FlowCall, has_body: impl Fn(&FlowCall) -> bool) -> Role {
        let named = |keys: &[String]| keys.iter().any(|key| names(key, call));
        if named(&sink.callees) {
            Role::Makes
        } else if named(&sink.appends.one) {
            Role::AddsOne
        } else if named(&sink.appends.each) {
            Role::AddsEach
        } else if has_body(call) {
            Role::Unknown
        } else {
            Role::Passes
        }
    }

    fn adds(self) -> bool {
        matches!(self, Role::AddsOne | Role::AddsEach)
    }
}

/// What's known of a command as it's built: the text of each argument that's a string literal, at
/// the position it would have in the sink's own call, and where the next one goes. Nothing after
/// where a shell's code goes is kept, so a function has few of these.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Built {
    literals: Vec<(usize, String)>,
    next: usize,
}

impl Built {
    /// The command once an argument with this text is added, for a shell's code at `code_at`.
    fn then(&self, text: Option<&str>, code_at: usize) -> Built {
        let kept = text.filter(|_| self.next < code_at);
        let kept = kept.map(|text| (self.next, text.to_owned()));
        Built {
            literals: self.literals.iter().cloned().chain(kept).collect(),
            next: (self.next + 1).min(code_at + 1),
        }
    }
}

/// The calls whose result reaches each call's receiver.
fn receivers(flows: &ValueFlows) -> MadeOn {
    let mut receivers = MadeOn::new();
    for hop in &flows.hops {
        let (
            Start::Result(from),
            End::Argument {
                call,
                position: 0,
                element: None,
            },
        ) = (hop.from, hop.to)
        else {
            continue;
        };
        receivers.entry(call).or_default().insert(from);
    }
    receivers
}

/// Every call made before this one on what it's made on.
fn ancestors(receivers: &MadeOn, call: usize) -> BTreeSet<usize> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![call];
    while let Some(at) = stack.pop() {
        for &before in receivers.get(&at).into_iter().flatten() {
            if seen.insert(before) {
                stack.push(before);
            }
        }
    }
    seen
}

/// What each of these calls was made on last. A borrow keeps what a binding held, so after
/// `cmd.arg("-c"); cmd.arg(script)` the command `new` made and the one the first `arg` gave back
/// both reach the second `arg`, and it's the one made after the other that counts.
fn last_made_on(receivers: &MadeOn, calls: &BTreeSet<usize>) -> MadeOn {
    let before: MadeOn = calls
        .iter()
        .map(|&call| (call, ancestors(receivers, call)))
        .collect();
    let made_after =
        |on: usize, other: usize| before.get(&other).is_some_and(|it| it.contains(&on));
    let last = |all: &BTreeSet<usize>| -> BTreeSet<usize> {
        let later = |on: usize| {
            all.iter()
                .any(|&other| other != on && made_after(on, other))
        };
        all.iter().copied().filter(|&on| !later(on)).collect()
    };

    let made_on = calls
        .iter()
        .filter_map(|call| Some((*call, receivers.get(call)?)));
    made_on.map(|(call, all)| (call, last(all))).collect()
}

/// One sink's commands in one function.
struct Commands<'a> {
    shell: &'a Shell,
    program_at: usize,
    calls: &'a [FlowCall],
    roles: Vec<Role>,
    last_made_on: MadeOn,
}

impl<'a> Commands<'a> {
    /// A sink's commands in a function, when its shell is told to run code and something there adds
    /// to one.
    fn of(
        sink: &'a Sink,
        flows: &'a ValueFlows,
        has_body: impl Fn(&FlowCall) -> bool,
    ) -> Option<Self> {
        let (shell, &program_at) = sink.shell.as_ref().zip(sink.arguments.first())?;
        if sink.appends.one.is_empty() && sink.appends.each.is_empty() {
            return None;
        }

        let calls = flows.calls.iter();
        let roles: Vec<Role> = calls.map(|call| Role::of(sink, call, &has_body)).collect();
        let adding: Vec<usize> = (0..roles.len()).filter(|&at| roles[at].adds()).collect();
        if adding.is_empty() {
            return None;
        }

        let receivers = receivers(flows);
        let before = adding.iter().flat_map(|&call| ancestors(&receivers, call));
        let reaching = adding.iter().copied().chain(before).collect();
        Some(Self {
            shell,
            program_at,
            calls: &flows.calls,
            roles,
            last_made_on: last_made_on(&receivers, &reaching),
        })
    }

    fn code_at(&self) -> usize {
        self.program_at + 2
    }

    /// The command a call the sink names makes: its program, and nothing added yet.
    fn made(&self, call: usize) -> Built {
        let literals = self.calls[call].literals.iter();
        let literals = literals.filter(|(at, _)| *at < self.code_at());
        Built {
            literals: literals.cloned().collect(),
            next: self.program_at + 1,
        }
    }

    /// What a call adds: for each argument, the element of a list it is and its text where it's a
    /// literal. Nothing is known after a call that could add anything.
    fn added(&self, call: usize) -> Option<Vec<(Option<usize>, Option<&str>)>> {
        let called = &self.calls[call];
        match self.roles[call] {
            Role::AddsOne => {
                let text = called.literals.iter().find(|(at, _)| *at == ADDED);
                Some(vec![(None, text.map(|(_, text)| text.as_str()))])
            }
            Role::AddsEach => {
                let (_, list) = called.lists.iter().find(|(at, _)| *at == ADDED)?;
                let elements = list.iter().enumerate();
                Some(
                    elements
                        .map(|(at, text)| (Some(at), text.as_deref()))
                        .collect(),
                )
            }
            Role::Makes | Role::Passes => Some(Vec::new()),
            Role::Unknown => None,
        }
    }

    /// What a call gives back, from what it was made on.
    fn after(&self, call: usize, before: Option<&BTreeSet<Built>>) -> BTreeSet<Built> {
        if self.roles[call] == Role::Makes {
            return BTreeSet::from([self.made(call)]);
        }
        let Some(added) = self.added(call) else {
            return BTreeSet::new();
        };
        let then = |built: &Built| {
            let added = added.iter();
            added.fold(built.clone(), |built, (_, text)| {
                built.then(*text, self.code_at())
            })
        };
        before.into_iter().flatten().map(then).collect()
    }

    /// What each call can be made on, over every way through the function. What's kept of a
    /// command is bounded, so this settles.
    fn built(&self) -> BTreeMap<usize, BTreeSet<Built>> {
        let mut built: BTreeMap<usize, BTreeSet<Built>> = BTreeMap::new();
        let mut changed = true;
        while changed {
            changed = false;
            for (&call, last) in &self.last_made_on {
                let on = last.iter().flat_map(|&on| self.after(on, built.get(&on)));
                let on: BTreeSet<Built> = on.collect();
                changed |= built.get(&call) != Some(&on);
                built.insert(call, on);
            }
        }
        built
    }

    /// Each argument a call adds where a shell runs it as code, by its call and element.
    fn code(&self) -> BTreeSet<(usize, Option<usize>)> {
        let mut code = BTreeSet::new();
        for (call, built) in self.built() {
            let Some(added) = self.added(call) else {
                continue;
            };
            for built in built {
                let mut at = built;
                for &(element, text) in &added {
                    if shell_code_at(self.shell, self.program_at, &at.literals) == Some(at.next) {
                        code.insert((call, element));
                    }
                    at = at.then(text, self.code_at());
                }
            }
        }
        code
    }
}

/// Where each command a function builds puts what a shell runs as code, for every sink whose
/// command is built a call at a time.
pub(super) fn shell_code(flows: &ValueFlows, has_body: impl Fn(&FlowCall) -> bool) -> ShellCode {
    let mut found = ShellCode::new();
    for sink in &Models::of(&flows.namespace).sinks {
        let Some(commands) = Commands::of(sink, flows, &has_body) else {
            continue;
        };
        let risk = Risk {
            risk: sink.risk.clone(),
            cwe: Some(sink.cwe.clone()),
        };
        for (call, element) in commands.code() {
            found.insert((call, ADDED, element), risk.clone());
        }
    }
    found
}
