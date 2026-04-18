#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentFlags {
    pub time: bool,
    pub model: bool,
    pub folder: bool,
    pub git: bool,
    pub context: bool,
    pub cost: bool,
    pub limits: bool,
    pub version: bool,
}

impl Default for SegmentFlags {
    fn default() -> Self {
        Self {
            time: true,
            model: true,
            folder: true,
            git: true,
            context: true,
            cost: true,
            limits: true,
            version: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    pub segments: SegmentFlags,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_all_segments_enabled() {
        let s = Settings::default();
        assert!(s.segments.time);
        assert!(s.segments.model);
        assert!(s.segments.folder);
        assert!(s.segments.git);
        assert!(s.segments.context);
        assert!(s.segments.cost);
        assert!(s.segments.limits);
        assert!(s.segments.version);
    }
}
