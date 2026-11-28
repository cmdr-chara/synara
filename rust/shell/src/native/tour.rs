//! App-tour startup gating and dismissal persistence.

use super::*;

impl Native {
    pub(super) fn feature_tour_installation(&self) -> Option<String> {
        self.settings_data
            .get("config")
            .and_then(|config| config.get("worktreesDir"))
            .and_then(Value::as_str)
            .filter(|path| !path.is_empty())
            .map(str::to_string)
    }

    /// Upstream FeatureTourDialog opens once per installation after startup
    /// surfaces settle. The native shell has no separate welcome flow yet, so
    /// the first successful server-config hydration is its startup gate.
    pub(super) fn maybe_start_feature_tour(&mut self) {
        if self.tour_startup_evaluated || self.tour_step.is_some() {
            return;
        }
        let Some(installation) = self.feature_tour_installation() else {
            return;
        };
        self.tour_startup_evaluated = true;
        if !self
            .tour_seen_installations
            .iter()
            .any(|seen| seen == &installation)
        {
            self.tour_step = Some(0);
        }
    }

    pub(super) fn dismiss_feature_tour(&mut self) {
        if self.tour_step.take().is_none() {
            return;
        }
        let Some(installation) = self.feature_tour_installation() else {
            return;
        };
        if !self
            .tour_seen_installations
            .iter()
            .any(|seen| seen == &installation)
        {
            self.tour_seen_installations.push(installation.clone());
        }
        crate::feature_tour::mark_seen(&installation);
    }
}
