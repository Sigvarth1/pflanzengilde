use crate::catalog::{Catalog, Species};
use crate::guild::Guild;

pub fn acidophile(plant: &Species) -> bool {
    [
        "cranberry",
        "lingonberry",
        "wintergreen",
        "tea-sinensis",
        "rhododendron",
        "blueberry",
    ]
    .iter()
    .any(|word| plant.id.contains(word))
}

pub fn calcicole(plant: &Species) -> bool {
    ["sage", "alfalfa", "sainfoin", "hellebore"]
        .iter()
        .any(|word| plant.id.contains(word))
}

pub fn acid_tree(tree: &Species) -> bool {
    matches!(
        tree.id.as_str(),
        "shrub-blueberry"
            | "shrub-rhododendron"
            | "tree-tea-sinensis"
            | "tree-tea-assamica"
            | "tree-chestnut"
    )
}

pub fn acid_intolerant(tree: &Species) -> bool {
    tree.unsuitable_soils.iter().any(|soil| soil == "ACIDIC")
}

pub fn ph_pair(a: &Species, b: &Species) -> bool {
    (acidophile(a) && calcicole(b)) || (calcicole(a) && acidophile(b))
}

impl Catalog {
    pub fn compatible_with_star(&self, plant: &Species, tree: &Species) -> bool {
        !(tree.juglone_producer && plant.juglone_sensitive
            || (acid_tree(tree) && calcicole(plant))
            || (!acid_tree(tree) && acid_intolerant(tree) && acidophile(plant))
            || self
                .pest_conflicts
                .iter()
                .any(|pest| pest.tree_ids.contains(&tree.id) && pest.host_ids.contains(&plant.id)))
    }
    pub fn incompatibilities(
        &self,
        plant: &Species,
        tree: &Species,
        peers: &[String],
    ) -> Vec<String> {
        let mut warnings = Vec::new();
        let tree_warning = |key| self.text(key).replace("{other}", &tree.name);
        if tree.juglone_producer && plant.juglone_sensitive {
            warnings.push(tree_warning("compatReasonJuglone"));
        }
        let acid_tree = acid_tree(tree);
        let acid_intolerant = !acid_tree && acid_intolerant(tree);
        let (acid, calc) = (acidophile(plant), calcicole(plant));
        if (acid_tree && calc) || (acid_intolerant && acid) {
            warnings.push(tree_warning("compatReasonEdaphic"));
        }
        for pest in &self.pest_conflicts {
            if pest.tree_ids.contains(&tree.id) && pest.host_ids.contains(&plant.id) {
                warnings
                    .push(tree_warning("compatReasonPestHost").replace("{conflict}", &pest.title));
            }
        }
        if warnings.is_empty() && (acid || calc) {
            if let Some(peer) = self.plants.iter().find(|peer| {
                peer.id != plant.id
                    && peers.contains(&peer.id)
                    && ((acid && calcicole(peer)) || (calc && acidophile(peer)))
            }) {
                warnings.push(
                    self.text("compatReasonEdaphic")
                        .replace("{other}", &peer.name),
                );
            }
        }
        warnings
    }

    pub fn guild_incompatibilities(&self, plant: &Species, guild: &Guild) -> Vec<String> {
        self.trees
            .iter()
            .find(|tree| tree.id == guild.tree_id)
            .map_or_else(Vec::new, |tree| {
                self.incompatibilities(plant, tree, &guild.plant_ids)
            })
    }

    pub fn set_tree(&self, guild: &mut Guild, tree_id: String) -> String {
        let Some(tree) = self.trees.iter().find(|tree| tree.id == tree_id) else {
            return String::new();
        };
        guild.tree_id = tree_id;
        let mut removed = Vec::new();
        let mut compatible = Vec::new();
        for id in std::mem::take(&mut guild.plant_ids) {
            let Some(plant) = self.plants.iter().find(|plant| plant.id == id) else {
                continue;
            };
            if self.incompatibilities(plant, tree, &compatible).is_empty() {
                compatible.push(id);
            } else {
                removed.push(plant.name.as_str());
            }
        }
        guild.plant_ids = compatible;
        if removed.is_empty() {
            String::new()
        } else {
            self.text("compatAutoRemovedToast")
                .replace("{count}", &removed.len().to_string())
                .replace("{star}", &tree.name)
                .replace("{names}", &removed.join(", "))
        }
    }
}
