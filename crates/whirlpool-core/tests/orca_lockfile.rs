use std::path::PathBuf;

#[test]
fn all_locked_orca_packages_are_apache_release_1_0_4() {
    let lock = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let value: toml::Value = std::fs::read_to_string(lock).unwrap().parse().unwrap();
    let packages = value["package"].as_array().unwrap();
    let orca: Vec<_> = packages
        .iter()
        .filter(|package| package["name"].as_str().unwrap().starts_with("orca_"))
        .collect();
    let names: std::collections::BTreeSet<_> = orca
        .iter()
        .map(|package| package["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["orca_whirlpools_core", "orca_whirlpools_macros"].into()
    );
    for package in orca {
        assert_eq!(package["version"].as_str(), Some("1.0.4"));
    }
}
