use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

pub static RAW_UAD_DATA: &str = include_str!("../../resources/uad_lists.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub list: String,
    pub description: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default, rename = "neededBy")]
    pub needed_by: Vec<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    pub removal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStats {
    pub total_packages: usize,
    pub recommended_count: usize,
    pub advanced_count: usize,
    pub expert_count: usize,
    pub unsafe_count: usize,
    pub oem_count: usize,
    pub google_count: usize,
    pub aosp_count: usize,
    pub carrier_count: usize,
    pub misc_count: usize,
}

static CATALOG: OnceLock<HashMap<String, PackageMetadata>> = OnceLock::new();

pub fn get_catalog() -> &'static HashMap<String, PackageMetadata> {
    CATALOG.get_or_init(|| {
        serde_json::from_str::<HashMap<String, PackageMetadata>>(RAW_UAD_DATA).unwrap_or_default()
    })
}

pub fn get_dataset_stats() -> DatasetStats {
    let catalog = get_catalog();
    let mut stats = DatasetStats {
        total_packages: catalog.len(),
        recommended_count: 0,
        advanced_count: 0,
        expert_count: 0,
        unsafe_count: 0,
        oem_count: 0,
        google_count: 0,
        aosp_count: 0,
        carrier_count: 0,
        misc_count: 0,
    };

    for meta in catalog.values() {
        match meta.removal.to_lowercase().as_str() {
            "recommended" => stats.recommended_count += 1,
            "advanced" => stats.advanced_count += 1,
            "expert" => stats.expert_count += 1,
            "unsafe" => stats.unsafe_count += 1,
            _ => {}
        }
        match meta.list.to_lowercase().as_str() {
            "oem" => stats.oem_count += 1,
            "google" => stats.google_count += 1,
            "aosp" => stats.aosp_count += 1,
            "carrier" => stats.carrier_count += 1,
            "misc" => stats.misc_count += 1,
            _ => {}
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_loads_successfully() {
        let catalog = get_catalog();
        assert!(!catalog.is_empty(), "Catalog should not be empty");
        assert!(
            catalog.len() >= 5000,
            "Catalog should contain at least 5000 packages"
        );

        // Test a known package
        if let Some(pkg) = catalog.get("org.lineageos.jelly") {
            assert_eq!(pkg.removal, "Recommended");
            assert_eq!(pkg.list, "Oem");
        }
    }

    #[test]
    fn test_catalog_stats() {
        let stats = get_dataset_stats();
        assert!(stats.total_packages >= 5000);
        assert!(stats.recommended_count > 0);
        assert!(stats.unsafe_count > 0);
    }
}
