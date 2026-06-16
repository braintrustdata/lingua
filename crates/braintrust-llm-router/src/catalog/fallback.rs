use std::collections::{HashMap, HashSet};

pub(super) fn build_equivalence_index(
    model_names: HashSet<String>,
    fallback_models: &HashMap<String, Vec<String>>,
) -> HashMap<String, Vec<String>> {
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for name in &model_names {
        adjacency.entry(name.clone()).or_default();
    }

    for (name, fallbacks) in fallback_models {
        if !model_names.contains(name) {
            continue;
        }
        for fallback_model in fallbacks {
            if !model_names.contains(fallback_model) {
                continue;
            }
            adjacency
                .entry(name.clone())
                .or_default()
                .push(fallback_model.clone());
            adjacency
                .entry(fallback_model.clone())
                .or_default()
                .push(name.clone());
        }
    }

    let mut visited = HashSet::new();
    let mut index = HashMap::new();
    for name in model_names {
        if visited.contains(&name) {
            continue;
        }

        let mut stack = vec![name.clone()];
        let mut component = Vec::new();
        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            component.push(current.clone());
            if let Some(neighbors) = adjacency.get(&current) {
                stack.extend(neighbors.iter().cloned());
            }
        }

        if component.len() <= 1 {
            continue;
        }
        component.sort();
        for member in &component {
            index.insert(
                member.clone(),
                component
                    .iter()
                    .filter(|other| *other != member)
                    .cloned()
                    .collect(),
            );
        }
    }

    index
}
