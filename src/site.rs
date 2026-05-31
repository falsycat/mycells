use crate::cell::Cell;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PageRef {
    pub id: String,
    pub slug: String,
    pub date: String,
    pub title: String,
    pub url: String,
}

impl PageRef {
    pub fn from_cell(cell: &Cell) -> Self {
        PageRef {
            id: cell.id.clone(),
            slug: cell.slug.clone(),
            date: cell.date.clone(),
            title: cell.title.clone(),
            url: cell.url(),
        }
    }
}

pub struct Site {
    cells: HashMap<String, Cell>,
    slug_to_id: HashMap<String, String>,
    backlinks: HashMap<String, Vec<String>>,
    // IDs of non-index cells, sorted descending (newest first)
    sorted_ids: Vec<String>,
}

impl Site {
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let dir_abs = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        let git_root = crate::git::find_repo_root(&dir_abs);

        let mut cells: HashMap<String, Cell> = HashMap::new();
        let mut slug_to_id: HashMap<String, String> = HashMap::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let content = std::fs::read_to_string(&path)?;
            if let Some(mut cell) = Cell::from_file(&filename, &content) {
                if let Some(ref root) = git_root {
                    let abs_file = dir_abs.join(&filename);
                    if let Ok(rel) = abs_file.strip_prefix(root) {
                        cell.history = crate::git::file_history(root, rel);
                    }
                }
                slug_to_id.insert(cell.slug.clone(), cell.id.clone());
                cells.insert(cell.id.clone(), cell);
            } else {
                eprintln!("warning: skipping {filename} (no H1 title or unrecognised filename)");
            }
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

        Ok(Site {
            cells,
            slug_to_id,
            backlinks,
            sorted_ids,
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
        self.sorted_ids
            .iter()
            .filter_map(|id| self.cells.get(id))
            .map(PageRef::from_cell)
            .collect()
    }

    pub fn backlink_pagerefs(&self, id: &str) -> Vec<PageRef> {
        self.backlinks
            .get(id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|bid| self.cells.get(bid))
                    .filter(|c| c.id != "index")
                    .map(PageRef::from_cell)
                    .collect()
            })
            .unwrap_or_default()
    }
}
