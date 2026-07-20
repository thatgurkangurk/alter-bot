use ferinth::Ferinth;
use std::collections::HashMap;

use anyhow::Result;

pub async fn are_on_version(
    ferinth: &Ferinth<()>,
    mods: Vec<String>,
    version: &str,
) -> Result<HashMap<String, bool>> {
    let mut results: HashMap<String, bool> = HashMap::new();

    let id_slices: Vec<&str> = mods.iter().map(|s| s.as_str()).collect();

    let projects = ferinth.project_get_multiple(&id_slices).await?;

    for project in projects {
        let name = project.title.clone();
        let is_on_version = project.game_versions.contains(&version.to_string());

        results.insert(name, is_on_version);
    }

    Ok(results)
}
