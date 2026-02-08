use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use uuid::Uuid;

use crate::storage::migrate::{load_legacy_requests, migrate_legacy};
use crate::storage::postman::{new_id, PostmanCollection, PostmanHeader, PostmanItem, PostmanRequest};
use crate::storage::project::{collection_path, ensure_storage_dir, find_project_root, requests_dir};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Project,
    Folder,
    Request,
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: Uuid,
    pub name: String,
    pub kind: NodeKind,
    pub parent_id: Option<Uuid>,
    pub children: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ProjectTree {
    pub root_id: Uuid,
    pub nodes: HashMap<Uuid, TreeNode>,
}

#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct CollectionStore {
    pub root: PathBuf,
    pub collection: PostmanCollection,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequestFile {
    pub id: String,
    pub parent_id: String,
    pub project_id: String,
    pub item: PostmanItem,
}

impl CollectionStore {
    pub fn load_or_init() -> Result<Self, String> {
        let root = find_project_root()
            .ok_or("Could not find project root. Run from a directory with .git, Cargo.toml, package.json, or create a .perseus folder.")?;
        let _ = ensure_storage_dir()?;
        let path = collection_path().ok_or("Could not find project root")?;

        let mut collection = if path.exists() {
            let contents =
                fs::read_to_string(&path).map_err(|e| format!("Failed to read collection: {}", e))?;
            serde_json::from_str::<PostmanCollection>(&contents)
                .map_err(|e| format!("Failed to parse collection: {}", e))?
        } else {
            let legacy = load_legacy_requests()?;
            let root_name = root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Perseus")
                .to_string();
            if !legacy.is_empty() {
                migrate_legacy(root_name, "Imported".to_string(), legacy)
            } else {
                let mut collection = PostmanCollection::new(root_name.clone());
                let project = PostmanItem::new_folder(root_name);
                collection.item.push(project);
                collection
            }
        };

        let mut changed = ensure_ids(&mut collection);
        changed |= sort_collection(&mut collection);

        let store = Self { root, collection };
        if !path.exists() || changed {
            store.save()?;
        }
        Ok(store)
    }

    pub fn save(&self) -> Result<(), String> {
        let _ = ensure_storage_dir()?;
        let path = collection_path().ok_or("Could not find project root")?;
        let json = serde_json::to_string_pretty(&self.collection)
            .map_err(|e| format!("Failed to serialize collection: {}", e))?;
        fs::write(path, json).map_err(|e| format!("Failed to write collection: {}", e))?;
        Ok(())
    }

    pub fn list_projects(&self) -> Vec<ProjectInfo> {
        self.collection
            .item
            .iter()
            .filter_map(|item| parse_uuid(&item.id).map(|id| ProjectInfo {
                id,
                name: item.name.clone(),
            }))
            .collect()
    }

    pub fn add_project(&mut self, name: String) -> Result<Uuid, String> {
        let project = PostmanItem::new_folder(name);
        let id = parse_uuid(&project.id).ok_or("Invalid project id")?;
        self.collection.item.push(project);
        sort_collection(&mut self.collection);
        Ok(id)
    }

    pub fn build_tree(&self, project_id: Uuid) -> Result<ProjectTree, String> {
        let project_item = find_item(&self.collection.item, &project_id.to_string())
            .ok_or("Project not found")?;
        let mut nodes = HashMap::new();

        let mut root_node = TreeNode {
            id: project_id,
            name: project_item.name.clone(),
            kind: NodeKind::Project,
            parent_id: None,
            children: Vec::new(),
        };

        for child in &project_item.item {
            if let Some(child_id) = parse_uuid(&child.id) {
                root_node.children.push(child_id);
                build_tree_node(child, project_id, &mut nodes);
            }
        }

        nodes.insert(project_id, root_node);

        Ok(ProjectTree {
            root_id: project_id,
            nodes,
        })
    }

    pub fn get_item(&self, id: Uuid) -> Option<&PostmanItem> {
        find_item(&self.collection.item, &id.to_string())
    }

    pub fn get_item_mut(&mut self, id: Uuid) -> Option<&mut PostmanItem> {
        find_item_mut(&mut self.collection.item, &id.to_string())
    }

    pub fn rename_item(&mut self, id: Uuid, name: String) -> Result<(), String> {
        let item = self
            .get_item_mut(id)
            .ok_or("Item not found for rename")?;
        item.name = name;
        sort_collection(&mut self.collection);
        Ok(())
    }

    pub fn delete_item(&mut self, id: Uuid) -> Result<(), String> {
        let (parent_items, index) = find_parent_vec_mut(&mut self.collection.item, &id.to_string())
            .ok_or("Item not found for delete")?;
        parent_items.remove(index);
        Ok(())
    }

    pub fn duplicate_item(&mut self, id: Uuid) -> Result<Uuid, String> {
        let (parent_items, index) = find_parent_vec_mut(&mut self.collection.item, &id.to_string())
            .ok_or("Item not found for duplicate")?;
        let clone = clone_with_new_ids(&parent_items[index]);
        let clone_id = parse_uuid(&clone.id).ok_or("Invalid cloned id")?;
        parent_items.insert(index + 1, clone);
        sort_collection(&mut self.collection);
        Ok(clone_id)
    }

    pub fn move_item(&mut self, id: Uuid, dest_id: Uuid) -> Result<(), String> {
        let (parent_items, index) = find_parent_vec_mut(&mut self.collection.item, &id.to_string())
            .ok_or("Item not found for move")?;
        let item = parent_items.remove(index);

        let dest = self
            .get_item_mut(dest_id)
            .ok_or("Destination not found")?;
        if dest.is_request() {
            return Err("Cannot move into a request".to_string());
        }
        dest.item.push(item);
        sort_collection(&mut self.collection);
        Ok(())
    }

    pub fn add_folder(&mut self, parent_id: Uuid, name: String) -> Result<Uuid, String> {
        let parent = self
            .get_item_mut(parent_id)
            .ok_or("Parent not found for add folder")?;
        if parent.is_request() {
            return Err("Cannot add folder inside a request".to_string());
        }
        let folder = PostmanItem::new_folder(name);
        let id = parse_uuid(&folder.id).ok_or("Invalid folder id")?;
        parent.item.push(folder);
        sort_collection(&mut self.collection);
        Ok(id)
    }

    pub fn add_request(
        &mut self,
        parent_id: Uuid,
        name: String,
        request: PostmanRequest,
    ) -> Result<Uuid, String> {
        let parent = self
            .get_item_mut(parent_id)
            .ok_or("Parent not found for add request")?;
        if parent.is_request() {
            return Err("Cannot add request inside a request".to_string());
        }
        let item = PostmanItem::new_request(name, request);
        let id = parse_uuid(&item.id).ok_or("Invalid request id")?;
        parent.item.push(item);
        sort_collection(&mut self.collection);
        Ok(id)
    }

    pub fn update_request(&mut self, id: Uuid, request: PostmanRequest) -> Result<(), String> {
        let item = self
            .get_item_mut(id)
            .ok_or("Item not found for update")?;
        item.request = Some(request);
        Ok(())
    }

    pub fn save_request_file(
        &self,
        request_id: Uuid,
        parent_id: Uuid,
        project_id: Uuid,
    ) -> Result<(), String> {
        let dir = match requests_dir() {
            Some(d) => d,
            None => return Err("Could not find project root".to_string()),
        };
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create request dir: {}", e))?;

        let item = self
            .get_item(request_id)
            .ok_or("Request not found")?
            .clone();

        let file = RequestFile {
            id: request_id.to_string(),
            parent_id: parent_id.to_string(),
            project_id: project_id.to_string(),
            item,
        };
        let json = serde_json::to_string_pretty(&file)
            .map_err(|e| format!("Failed to serialize request file: {}", e))?;
        let path = dir.join(format!("{}.json", request_id));
        fs::write(path, json).map_err(|e| format!("Failed to write request file: {}", e))?;
        Ok(())
    }

    pub fn write_all_request_files(&self) -> Result<(), String> {
        let dir = match requests_dir() {
            Some(d) => d,
            None => return Err("Could not find project root".to_string()),
        };
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create request dir: {}", e))?;

        let mut seen: HashSet<String> = HashSet::new();
        let mut stack: Vec<(&PostmanItem, Option<Uuid>, Option<Uuid>)> = Vec::new();
        for project in &self.collection.item {
            if let Some(project_id) = parse_uuid(&project.id) {
                stack.push((project, None, Some(project_id)));
            }
        }

        while let Some((item, parent_id, project_id)) = stack.pop() {
            if item.is_request() {
                if let (Some(pid), Some(proj_id), Some(id)) =
                    (parent_id, project_id, parse_uuid(&item.id))
                {
                    seen.insert(id.to_string());
                    let file = RequestFile {
                        id: id.to_string(),
                        parent_id: pid.to_string(),
                        project_id: proj_id.to_string(),
                        item: item.clone(),
                    };
                    let json = serde_json::to_string_pretty(&file)
                        .map_err(|e| format!("Failed to serialize request file: {}", e))?;
                    let path = dir.join(format!("{}.json", id));
                    fs::write(path, json)
                        .map_err(|e| format!("Failed to write request file: {}", e))?;
                }
            }

            if !item.item.is_empty() {
                let current_id = parse_uuid(&item.id);
                for child in &item.item {
                    stack.push((child, current_id, project_id));
                }
            }
        }

        let entries = fs::read_dir(&dir)
            .map_err(|e| format!("Failed to read request dir: {}", e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if !seen.contains(stem) {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        }

        Ok(())
    }
}

impl ProjectTree {
    pub fn node(&self, id: Uuid) -> Option<&TreeNode> {
        self.nodes.get(&id)
    }

    pub fn is_descendant(&self, ancestor: Uuid, child: Uuid) -> bool {
        let mut current = Some(child);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.nodes.get(&id).and_then(|n| n.parent_id);
        }
        false
    }

    pub fn path_for(&self, id: Uuid) -> Vec<String> {
        let mut segments = Vec::new();
        let mut current = Some(id);
        while let Some(node_id) = current {
            if let Some(node) = self.nodes.get(&node_id) {
                segments.push(node.name.clone());
                current = node.parent_id;
            } else {
                break;
            }
        }
        segments.reverse();
        segments
    }
}

fn build_tree_node(item: &PostmanItem, parent_id: Uuid, nodes: &mut HashMap<Uuid, TreeNode>) {
    let id = match parse_uuid(&item.id) {
        Some(id) => id,
        None => return,
    };
    let kind = if item.request.is_some() {
        NodeKind::Request
    } else {
        NodeKind::Folder
    };
    let mut node = TreeNode {
        id,
        name: item.name.clone(),
        kind,
        parent_id: Some(parent_id),
        children: Vec::new(),
    };

    for child in &item.item {
        if let Some(child_id) = parse_uuid(&child.id) {
            node.children.push(child_id);
            build_tree_node(child, id, nodes);
        }
    }
    nodes.insert(id, node);
}

fn ensure_ids(collection: &mut PostmanCollection) -> bool {
    let mut changed = false;
    if collection.info.postman_id.trim().is_empty()
        || parse_uuid(&collection.info.postman_id).is_none()
    {
        collection.info.postman_id = new_id();
        changed = true;
    }

    for item in &mut collection.item {
        changed |= ensure_item_ids(item);
    }
    changed
}

fn ensure_item_ids(item: &mut PostmanItem) -> bool {
    let mut changed = false;
    if item.id.trim().is_empty() || parse_uuid(&item.id).is_none() {
        item.id = new_id();
        changed = true;
    }
    for child in &mut item.item {
        changed |= ensure_item_ids(child);
    }
    changed
}

fn sort_collection(collection: &mut PostmanCollection) -> bool {
    sort_items(&mut collection.item)
}

fn sort_items(items: &mut Vec<PostmanItem>) -> bool {
    let before: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
    items.sort_by(|a, b| {
        let an = a.name.to_lowercase();
        let bn = b.name.to_lowercase();
        an.cmp(&bn).then_with(|| a.id.cmp(&b.id))
    });
    let mut changed = before != items.iter().map(|i| i.id.clone()).collect::<Vec<_>>();
    for item in items.iter_mut() {
        changed |= sort_items(&mut item.item);
    }
    changed
}

fn parse_uuid(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok()
}

fn find_item<'a>(items: &'a [PostmanItem], id: &str) -> Option<&'a PostmanItem> {
    for item in items {
        if item.id == id {
            return Some(item);
        }
        if let Some(found) = find_item(&item.item, id) {
            return Some(found);
        }
    }
    None
}

fn find_item_mut<'a>(items: &'a mut Vec<PostmanItem>, id: &str) -> Option<&'a mut PostmanItem> {
    for item in items.iter_mut() {
        if item.id == id {
            return Some(item);
        }
        if let Some(found) = find_item_mut(&mut item.item, id) {
            return Some(found);
        }
    }
    None
}

fn find_parent_vec_mut<'a>(
    items: &'a mut Vec<PostmanItem>,
    id: &str,
) -> Option<(&'a mut Vec<PostmanItem>, usize)> {
    let path = find_item_path(items, id)?;
    if path.is_empty() {
        return None;
    }
    let mut current = items;
    for idx in &path[..path.len() - 1] {
        current = &mut current[*idx].item;
    }
    let index = *path.last().unwrap();
    Some((current, index))
}

fn find_item_path(items: &[PostmanItem], id: &str) -> Option<Vec<usize>> {
    for (index, item) in items.iter().enumerate() {
        if item.id == id {
            return Some(vec![index]);
        }
        if let Some(mut path) = find_item_path(&item.item, id) {
            path.insert(0, index);
            return Some(path);
        }
    }
    None
}

fn clone_with_new_ids(item: &PostmanItem) -> PostmanItem {
    let mut clone = item.clone();
    clone.id = new_id();
    clone.item = clone
        .item
        .iter()
        .map(clone_with_new_ids)
        .collect();
    clone
}

pub fn parse_headers(raw: &str) -> Vec<PostmanHeader> {
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let mut parts = trimmed.splitn(2, ':');
            let key = parts.next()?.trim();
            let value = parts.next().unwrap_or("").trim();
            if key.is_empty() {
                return None;
            }
            Some(PostmanHeader {
                key: key.to_string(),
                value: value.to_string(),
                disabled: None,
            })
        })
        .collect()
}
