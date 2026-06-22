#[derive(Debug, Default)]
pub struct DirectoryLoadingBar {
    visible: bool,
    fraction: f32,
    completing: bool,
    complete_hold: f32,
}

impl DirectoryLoadingBar {
    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn fraction(&self) -> f32 {
        self.fraction.clamp(0.0, 1.0)
    }

    /// Advance progress toward `progress` while `loading`, then finish when idle.
    /// When `progress` is `None`, eases toward 0.9 (indeterminate phase).
    pub fn update(&mut self, loading: bool, progress: Option<f32>, dt: f32) -> bool {
        let dt = dt.clamp(0.0, 0.05);

        if loading {
            if !self.visible {
                self.fraction = 0.05;
            }
            self.visible = true;
            self.completing = false;
            self.complete_hold = 0.0;

            let target = progress.map(|value| value.clamp(0.05, 0.92)).unwrap_or(0.9);
            self.fraction += (target - self.fraction) * (1.0 - (-8.0 * dt).exp());
            return true;
        }

        if !self.visible {
            return false;
        }

        if !self.completing {
            self.completing = true;
        }

        self.fraction += (1.0 - self.fraction) * (1.0 - (-12.0 * dt).exp());
        if self.fraction > 0.995 {
            self.fraction = 1.0;
            self.complete_hold += dt;
            if self.complete_hold > 0.15 {
                self.visible = false;
                self.fraction = 0.0;
                self.completing = false;
                self.complete_hold = 0.0;
            }
        }

        true
    }
}
