use zotron_types::AcademicZhHit;

#[test]
fn academic_zh_jsonl_fixture_deserializes_into_typed_hits() {
    let fixture = include_str!("../../../fixtures/academic_zh_hits.jsonl");
    let hits = fixture
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str::<AcademicZhHit>)
        .collect::<Result<Vec<_>, _>>()
        .expect("fixture must match the typed academic-zh hit contract");

    assert_eq!(hits.len(), 9);
    assert_eq!(hits[0].item_key, "X6LYTXEJ");
    assert_eq!(hits[0].authors, vec!["濮双羽", "赵洪进"]);
    assert_eq!(hits[0].chunk_key, "NBUVZGWJ:c2");
    assert_eq!(hits[0].block_keys[0], "NBUVZGWJ:p0:b8");
    assert_eq!(
        hits[1].doi.as_deref(),
        Some("10.19641/j.cnki.42-1290/f.2021.03.013")
    );
}
