//! Immediate helper callbacks retain eager defaults and Some-only helper effects.
use super::*;
use crate::helper_option_callbacks::{Checked, ENVIRONMENT, SELECTOR};

impl Builder<'_> {
    pub(super) fn helper_callback_select(
        &mut self,
        checked: &Checked<'_>,
        saved: &str,
        flow: Flow,
    ) -> Flow {
        let created = self.emit(
            Operation {
                construct_span: checked.arguments.call.span,
                construct: Some(ENVIRONMENT.into()),
                role: "callback_environment".into(),
                ty: "unit".into(),
                ..template(checked.arguments.callback, NodeCategory::Computation)
            },
            flow,
        );

        self.emit(
            Operation {
                construct: Some(SELECTOR.into()),
                role: "callback_selector".into(),
                operands: vec![saved.into()],
                ty: "bool".into(),
                ..template(checked.arguments.call, NodeCategory::Control)
            },
            created,
        )
    }
}
