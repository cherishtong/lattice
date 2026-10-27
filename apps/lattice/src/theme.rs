#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    WuweiDark,
    WuweiLight,
    MatrixDark,
    MatrixLight,
}

impl Theme {
    pub const ALL: [Self; 4] = [
        Self::WuweiDark,
        Self::WuweiLight,
        Self::MatrixDark,
        Self::MatrixLight,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::WuweiDark => "wuwei-dark",
            Self::WuweiLight => "wuwei-light",
            Self::MatrixDark => "matrix-dark",
            Self::MatrixLight => "matrix-light",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "wuwei-dark" | "dark" => Some(Self::WuweiDark),
            "wuwei-light" | "light" => Some(Self::WuweiLight),
            "matrix-dark" | "midnight" => Some(Self::MatrixDark),
            "matrix-light" => Some(Self::MatrixLight),
            _ => None,
        }
    }

    pub const fn is_light(self) -> bool {
        matches!(self, Self::WuweiLight | Self::MatrixLight)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::WuweiDark => "无为 · 暗",
            Self::WuweiLight => "无为 · 纸",
            Self::MatrixDark => "矩阵 · 暗",
            Self::MatrixLight => "矩阵 · 白",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::WuweiDark => "墨色纸感，朱砂强调，衬线标题",
            Self::WuweiLight => "宣纸底色，墨色文字，朱砂点缀",
            Self::MatrixDark => "黑底荧光绿，等宽字形，低亮度",
            Self::MatrixLight => "白底森林绿，清爽高对比",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_has_a_unique_stable_id() {
        let mut ids = Theme::ALL.map(Theme::id).to_vec();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Theme::ALL.len());
        assert!(Theme::ALL.iter().all(|theme| !theme.id().is_empty()));
        assert!(
            Theme::ALL
                .iter()
                .all(|theme| Theme::from_id(theme.id()) == Some(*theme))
        );
        assert_eq!(Theme::from_id("unknown"), None);
    }

    #[test]
    fn legacy_theme_ids_migrate_to_new_families() {
        assert_eq!(Theme::from_id("dark"), Some(Theme::WuweiDark));
        assert_eq!(Theme::from_id("light"), Some(Theme::WuweiLight));
        assert_eq!(Theme::from_id("midnight"), Some(Theme::MatrixDark));
    }
}
