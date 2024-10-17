//! NOT YET IMPLEMENTED

use std::{
    path::PathBuf,
    collections::HashMap
};

use eaf_rs::{eaf::Eaf, Tier};

#[derive(Debug, Default)]
/// HashMap<TIER_ID, CHILD_TIER_IDs>
// struct TierTree(HashMap<TierType, Vec<TierType>>);
struct TierTree(HashMap<String, Vec<String>>);

// #[derive(Debug, Default)]
// struct TierTree2{

// };

pub struct Node{
    node: Box<Self>,
    depth: usize,
    data: HashMap<String, Vec<String>>
}

impl Node {
    pub fn new(eaf: &Eaf) {
        let main_tiers = eaf.tiers.iter().filter(|t| !t.is_ref()).collect::<Vec<_>>();
        let ref_tiers = eaf.tiers.iter().filter(|t| t.is_ref()).collect::<Vec<_>>();
    }
}

// #[derive(Debug, Clone, Hash, PartialEq, Eq)]
// enum TierType {
//     Main(String),
//     Ref(String)
// }

impl TierTree {
    fn new2(eaf: &Eaf) {
        let mut seen: Vec<&str> = Vec::new();
        let depth = 0;

        for tier in eaf.main_tiers().iter() {
            for child in eaf.child_tiers(&tier.tier_id).iter() {
                
            }
        }
    }

    fn new(eaf: &Eaf) -> Self {
        // Get all tier IDs, incl ref tiers, since these may in turn have ref tiers
        let mut tiers: HashMap<String, Vec<String>> = eaf.tiers.iter()
            .map(|t| (t.tier_id.to_owned(), Vec::new()))
            .collect();

        // Get all REF_TIER IDs
        for ref_tier in eaf.tiers.iter().filter(|t| t.is_ref()) {
            tiers.entry(ref_tier.parent_ref.as_deref().unwrap().to_owned())
                .or_insert(Vec::new()).push(ref_tier.tier_id.to_owned());
        }

        let mut tree = Self(tiers);
        tree.prune();

        tree
    }

    // fn new2(eaf: &AnnotationDocument) {
    //     let mut main_tiers = Vec::new();
    //     let mut ref_tiers = Vec::new();
    //     eaf.tiers.iter().for_each(|t| match t.is_ref() {
    //         true => 
    //     })
    // }

    fn remove(&mut self, key: &str) -> Option<Vec<String>> {
        self.0.remove(key)
    }

    /// Removes tier ID entries that are values for other tier IDs (Keys).
    fn prune(&mut self) {
        let keys = self.0.iter()
            .map(|(k, _)| k.to_owned())
            .collect::<Vec<_>>();
        let mut values = self.0.iter()
            .flat_map(|(_, v)| v.to_owned())
            .collect::<Vec<_>>();
        values.sort();
        values.dedup();

        for key in keys.iter() {
            // Check if referred tier ID...
            if values.contains(key) {
                // ... and if that referred tier ID has no children (value len = 0)...
                if let Some(val) = self.0.get(key) {
                    if val.len() == 0 {
                        // remove the entry entirely, since it already exists as value
                        self.remove(key);
                    }
                }
            }
        }
    }

    fn print(&self, tier_id: Option<&str>, depth: usize) {
        let indent = depth * 3;
        match tier_id {
            Some(t_id) => {
                let mut children = self.get(t_id);
                children.sort();
                let len = children.len();
                for (i, id) in children.iter().enumerate() {
                    match len {
                        1 => println!("{}╰─ {}", " ".repeat(indent), id),
                        _ => {
                            if i == len - 1 {
                                println!("{}╰─ {}", " ".repeat(indent), id)
                            } else {
                                println!("{}├─ {}", " ".repeat(indent), id)
                            }
                        }
                    }
                    self.print(Some(id), depth + 1);
                }
            }
            None => {
                let mut keys = self.0.keys().collect::<Vec<_>>();
                keys.sort();
                for id in keys.iter() {
                    println!("{}", id);
                    self.print(Some(id), 0);
                }
            }
        }
    }
    // fn print(eaf: &AnnotationDocument) {
    //     for tier in eaf.tiers.iter() {
    //         println!("TIER ID {}", tier.tier_id);
    //         let ref_tiers = eaf.child_tiers(&tier.tier_id)
    //             .iter()
    //             .map(|t| t.tier_id.as_str())
    //             .collect::<Vec<_>>();
    //         if !ref_tiers.is_empty() {
    //             println!("{:#?}", ref_tiers);
    //         }
    //     }
    // }

    // fn print(&self, depth: usize) {
    //     let mut children_with_children: Vec<String> = Vec::new();
    //     for val in self.0.values().iter()
    // }

    // fn get(self, tier_id: &str) -> Vec<TierType> {
    fn get(&self, tier_id: &str) -> Vec<String> {
        self.0.get(tier_id).cloned().unwrap_or_default()
        // self.0.iter()
        //     .find(|(t_type, _)| {
        //         match t_type {
        //             TierType::Main(id) => tier_id == id.as_str(),
        //             TierType::Ref(id) => tier_id == id.as_str(),
        //         }
        //     })
        //     .map(|(_, v)| v.to_owned())
        //     .unwrap_or_default()
    }

    fn len(self, tier_id: &str) -> usize {
        self.get(tier_id).len()
    }
}

pub fn run(args: &clap::ArgMatches) -> std::io::Result<()> {
    let path: &PathBuf = args.get_one("eaf").unwrap();
    let eaf = match Eaf::read(path) {
        Ok(f) => f,
        Err(err) => {
            println!("(!) Failed to parse '{}': {err}", path.display());
            std::process::exit(1)
        }
    };

    let tree = TierTree::new(&eaf);
    tree.print(None, 0);

    // TierTree::print(&eaf);

    // for (key, val) in tree.0.iter() {
    //     println!("{key:?}\n  {val:#?}");
    // }

    Ok(())
}