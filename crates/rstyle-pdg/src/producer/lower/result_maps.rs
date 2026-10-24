//! Each branch moves its payload into a newly typed Result; the input is saved once.
use super::*;
use crate::result_maps::CheckedMap;

struct SavedMap {
    selector: Flow,
    input: String,
}

struct MapBranch<'a> {
    checked: &'a CheckedMap<'a>,
    saved: &'a SavedMap,
    error: bool,
}

impl Builder<'_> {
    pub(super) fn result_map(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let checked = crate::result_maps::verify(node, self.facts)
            .expect("complete checked constructor Result chain");

        let saved = self.prepare_map(&checked, flow);
        let arms = self.map_arms(&checked, &saved);
        let controller = saved.selector.value.expect("saved Result selector");

        self.join(node, controller, arms)
    }

    fn prepare_map(&mut self, checked: &CheckedMap<'_>, flow: Flow) -> SavedMap {
        let evaluated = self.expression(checked.receiver, flow);

        self.save_map(checked, evaluated)
    }

    fn map_arms(&mut self, checked: &CheckedMap<'_>, saved: &SavedMap) -> Arms {
        let error = self.map_branch(MapBranch {
            checked,
            saved,
            error: true,
        });
        let success = self.map_branch(MapBranch {
            checked,
            saved,
            error: false,
        });

        Arms {
            yes: error,
            no: success,
        }
    }

    fn save_map(&mut self, checked: &CheckedMap<'_>, evaluated: Flow) -> SavedMap {
        let input = evaluated.value.clone().expect("saved transport Result");
        let selector = self.emit(
            Operation {
                ty: "bool".into(),
                construct: Some("map.result-is-err".into()),
                role: checked.role().into(),
                operands: vec![input.clone()],
                ..template(checked.source, NodeCategory::Control)
            },
            evaluated,
        );
        SavedMap { selector, input }
    }

    fn map_branch(&mut self, branch: MapBranch<'_>) -> Flow {
        let checked = branch.checked;
        let payload = self.map_payload(&branch);
        let value = if branch.error == checked.maps_error {
            self.map_constructor(checked, payload)
        } else {
            payload
        };

        let input = value.value.clone().expect("single moved branch payload");
        self.emit(
            Operation {
                ty: crate::data_types::token(&checked.output).expect("checked output Result"),
                construct: Some(
                    if branch.error {
                        "map.result-wrap-err"
                    } else {
                        "map.result-wrap-ok"
                    }
                    .into(),
                ),
                operands: vec![input],
                role: checked.role().into(),
                ..template(checked.source, NodeCategory::Computation)
            },
            value,
        )
    }

    fn map_payload(&mut self, branch: &MapBranch<'_>) -> Flow {
        let index = usize::from(branch.error);
        let flow = arm(
            &branch.saved.selector,
            if branch.error { "true" } else { "false" },
        );
        self.emit(
            Operation {
                ty: crate::data_types::token(branch.checked.payload_types()[index])
                    .expect("checked branch payload"),
                construct: Some(
                    if branch.error {
                        "map.result-err-payload"
                    } else {
                        "map.result-ok-payload"
                    }
                    .into(),
                ),
                operands: vec![branch.saved.input.clone()],
                role: branch.checked.role().into(),
                ..template(branch.checked.source, NodeCategory::Computation)
            },
            flow,
        )
    }

    fn map_constructor(&mut self, checked: &CheckedMap<'_>, flow: Flow) -> Flow {
        let input = flow.value.clone().expect("selected constructor field");
        self.emit(
            Operation {
                ty: crate::data_types::token(checked.constructed_type())
                    .expect("checked unary constructor output"),
                construct: Some("map.constructor-field".into()),
                operands: vec![input],
                role: checked.role().into(),
                ..template(checked.callback, NodeCategory::Computation)
            },
            flow,
        )
    }
}
