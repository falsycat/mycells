use crate::cell::Cell;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PageRef {
    pub id: String,
    pub slug: String,
    pub date: String,
    pub title: String,
    pub url: String,
    pub tags: Vec<String>,
}

impl PageRef {
    pub fn from_cell(cell: &Cell) -> Self {
        PageRef {
            id: cell.id.clone(),
            slug: cell.slug.clone(),
            date: cell.created_date().to_string(),
            title: cell.title.clone(),
            url: cell.url(),
            tags: cell.tags.clone(),
        }
    }
}

pub struct Site {
    cells: HashMap<String, Cell>,
    slug_to_id: HashMap<String, String>,
    backlinks: HashMap<String, Vec<String>>,
    // IDs of non-index cells, sorted descending (newest first), capped at 20
    recent_ids: Vec<String>,
}

impl Site {
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let dir_abs = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        let git_root = crate::git::find_repo_root(&dir_abs);

        // Collect directory entries first (sequential — OS I/O).
        let entries: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().and_then(|x| x.to_str()) == Some("md")
            })
            .collect();

        // Parse files in parallel with Rayon.
        let parsed: Vec<(String, Cell)> = entries
            .par_iter()
            .filter_map(|entry| {
                let path = entry.path();
                let filename = path.file_name()?.to_str()?.to_string();
                let content = std::fs::read_to_string(&path).ok()?;
                let mut cell = Cell::from_file(&filename, &content)
                    .or_else(|| {
                        eprintln!("warning: skipping {filename} (no H1 title or unrecognised filename)");
                        None
                    })?;

                if let Some(ref root) = git_root {
                    let abs_file = dir_abs.join(&filename);
                    if let Ok(rel) = abs_file.strip_prefix(root) {
                        cell.history = crate::git::file_history(root, rel);
                    }
                }
                Some((cell.id.clone(), cell))
            })
            .collect();

        let mut cells: HashMap<String, Cell> = HashMap::with_capacity(parsed.len());
        let mut slug_to_id: HashMap<String, String> = HashMap::with_capacity(parsed.len());

        for (id, cell) in parsed {
            slug_to_id.insert(cell.slug.clone(), id.clone());
            cells.insert(id, cell);
        }

        let mut backlinks: HashMap<String, Vec<String>> = HashMap::new();
        for cell in cells.values() {
            for target_id in &cell.outlinks {
                backlinks
                    .entry(target_id.clone())
                    .or_default()
                    .push(cell.id.clone());
            }
        }

        let mut sorted_ids: Vec<String> = cells
            .keys()
            .filter(|id| *id != "index")
            .cloned()
            .collect();
        sorted_ids.sort_by(|a, b| b.cmp(a));
        let recent_ids = sorted_ids.into_iter().take(20).collect();

        Ok(Site {
            cells,
            slug_to_id,
            backlinks,
            recent_ids,
        })
    }

    pub fn get_by_id(&self, id: &str) -> Option<&Cell> {
        self.cells.get(id)
    }

    pub fn get_by_slug(&self, slug: &str) -> Option<&Cell> {
        let id = self.slug_to_id.get(slug)?;
        self.cells.get(id)
    }

    pub fn all_cells(&self) -> impl Iterator<Item = &Cell> {
        self.cells.values()
    }

    pub fn recent_pagerefs(&self) -> Vec<PageRef> {
        self.recent_ids
            .iter()
            .filter_map(|id| self.cells.get(id))
            .map(PageRef::from_cell)
            .collect()
    }

    pub fn backlink_pagerefs(&self, id: &str) -> Vec<PageRef> {
        let Some(ids) = self.backlinks.get(id) else {
            return vec![];
        };
        let mut seen = std::collections::HashSet::new();
        ids.iter()
            .filter(|bid| seen.insert(bid.as_str()))
            .filter_map(|bid| self.cells.get(bid.as_str()))
            .filter(|c| c.id != "index")
            .map(PageRef::from_cell)
            .collect()
    }
}
