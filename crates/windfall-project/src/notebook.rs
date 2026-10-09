//! Saved plain-text song notes. Pages have no playback or executable behavior.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::CommandError;

pub const MAX_NOTEBOOK_PAGES: usize = 8;
pub const MAX_NOTEBOOK_TITLE_BYTES: usize = 128;
pub const MAX_NOTEBOOK_BODY_BYTES: usize = 16_384;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NotebookPage {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Notebook {
    #[serde(default)]
    pub pages: Vec<NotebookPage>,
}

impl Notebook {
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    pub fn check(&self) -> Result<(), CommandError> {
        if self.pages.len() > MAX_NOTEBOOK_PAGES {
            return Err(CommandError::invalid("a notebook can have at most 8 pages"));
        }
        for page in &self.pages {
            if page.title.len() > MAX_NOTEBOOK_TITLE_BYTES {
                return Err(CommandError::invalid(
                    "a notebook page title must be at most 128 UTF-8 bytes",
                ));
            }
            if page.body.len() > MAX_NOTEBOOK_BODY_BYTES {
                return Err(CommandError::invalid(
                    "a notebook page body must be at most 16384 UTF-8 bytes",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn normalized(mut self) -> Result<Self, CommandError> {
        for page in &mut self.pages {
            page.title = page.title.trim().to_owned();
        }
        self.check()?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Document, Project};

    fn replace(
        document: &mut Document,
        pages: Vec<NotebookPage>,
    ) -> Result<crate::Applied, CommandError> {
        document.dispatch(
            Command::ReplaceNotebook {
                notebook: Notebook { pages },
            },
            None,
        )
    }

    #[test]
    fn notebook_default_omission_and_round_trip() {
        let project = Project::new("Song");
        let json = serde_json::to_value(&project).unwrap();
        assert!(json.get("notebook").is_none());
        let restored: Project = serde_json::from_value(json).unwrap();
        assert_eq!(restored, project);
        assert!(restored.notebook.pages.is_empty());
        assert_eq!(
            serde_json::from_str::<Notebook>("{}").unwrap(),
            Notebook::default()
        );

        let mut document = Document::new(project);
        replace(
            &mut document,
            vec![NotebookPage {
                title: "Verse".into(),
                body: "  First line\nSecond line  ".into(),
            }],
        )
        .unwrap();
        let json = serde_json::to_value(document.project()).unwrap();
        assert_eq!(json["notebook"]["pages"][0]["title"], "Verse");
        assert_eq!(
            serde_json::from_value::<Project>(json).unwrap(),
            *document.project()
        );
    }

    #[test]
    fn notebook_byte_caps_trim_titles_and_preserve_body() {
        let mut document = Document::new(Project::new("Song"));
        let page = NotebookPage {
            title: format!("  {}  ", "é".repeat(MAX_NOTEBOOK_TITLE_BYTES / 2)),
            body: "é".repeat(MAX_NOTEBOOK_BODY_BYTES / 2),
        };
        replace(&mut document, vec![page.clone()]).unwrap();
        assert_eq!(
            document.project().notebook.pages[0].title,
            page.title.trim()
        );
        for oversized in [
            NotebookPage {
                title: format!("{}x", page.title.trim()),
                ..page.clone()
            },
            NotebookPage {
                body: format!("{}x", page.body),
                ..page.clone()
            },
            NotebookPage {
                body: format!("{} ", page.body),
                ..page.clone()
            },
        ] {
            let before = document.snapshot(None);
            assert!(matches!(
                replace(&mut document, vec![oversized]),
                Err(CommandError::Invalid(_))
            ));
            assert_eq!(document.snapshot(None), before);
        }
        replace(
            &mut document,
            vec![NotebookPage {
                title: "  Verse  ".into(),
                body: "  \n<script>plain text</script>  ".into(),
            }],
        )
        .unwrap();
        assert_eq!(document.project().notebook.pages[0].title, "Verse");
        assert_eq!(
            document.project().notebook.pages[0].body,
            "  \n<script>plain text</script>  "
        );
    }

    #[test]
    fn notebook_page_cap_is_atomic_and_checked_on_load() {
        let mut document = Document::new(Project::new("Song"));
        replace(
            &mut document,
            vec![NotebookPage::default(); MAX_NOTEBOOK_PAGES],
        )
        .unwrap();
        let before = document.snapshot(None);
        assert!(matches!(
            replace(
                &mut document,
                vec![NotebookPage::default(); MAX_NOTEBOOK_PAGES + 1]
            ),
            Err(CommandError::Invalid(_))
        ));
        assert_eq!(document.snapshot(None), before);
        let mut invalid = document.project().clone();
        invalid.notebook.pages.push(NotebookPage::default());
        assert!(invalid.check().is_err());
        invalid.notebook.pages.pop();
        invalid.notebook.pages[0].body = "x".repeat(MAX_NOTEBOOK_BODY_BYTES + 1);
        assert!(invalid.check().is_err());
    }

    #[test]
    fn notebook_replace_is_one_undo_step_and_patches_clear_the_book() {
        let project = Project::new("Song");
        let settings = project.settings.clone();
        let mut document = Document::new(project);
        let applied = replace(
            &mut document,
            vec![
                NotebookPage {
                    title: "Verse".into(),
                    body: "Lyrics".into(),
                },
                NotebookPage {
                    title: "Mix".into(),
                    body: "Notes".into(),
                },
            ],
        )
        .unwrap();
        assert!(applied.touched.notebook);
        assert!(!applied.touched.settings);
        assert_eq!(document.history().entries.len(), 1);
        let saved = document.project().notebook.clone();
        assert_eq!(
            document.patch(&applied.touched).notebook,
            Some(saved.clone())
        );
        let touched = document.undo().unwrap();
        assert!(document.project().notebook.is_empty());
        assert_eq!(document.patch(&touched).notebook, Some(Notebook::default()));
        assert!(!document.is_dirty());
        assert!(document.undo().is_none());
        document.redo().unwrap();
        assert_eq!(document.project().notebook, saved);
        replace(&mut document, vec![]).unwrap();
        assert!(document.project().notebook.is_empty());
        document.undo().unwrap();
        assert_eq!(document.project().notebook, saved);
        assert_eq!(document.project().settings, settings);
    }

    #[test]
    fn notebook_unchanged_replacement_preserves_redo() {
        let mut document = Document::new(Project::new("Song"));
        replace(&mut document, vec![NotebookPage::default()]).unwrap();
        document.undo().unwrap();
        let before = document.snapshot(None);
        assert!(replace(&mut document, vec![]).unwrap().touched.is_empty());
        assert_eq!(document.snapshot(None), before);
        assert!(document.redo().is_some());
    }
}
