use super::*;

#[test]
fn json_lines_parse_and_fences_and_arrays_are_accepted() {
    let answer = "```json\n{\"i\": 1, \"label\": \"Keep\", \"reason\": \"Tax files\"}\n{\"i\":0,\"label\":\"safe\",\"reason\":\"Cache\"},\n```";
    assert_eq!(
        parse(answer, 3),
        Ok(vec![
            (1, Label::Keep, "Tax files".into()),
            (0, Label::Safe, "Cache".into())
        ])
    );
    let array = r#"[{"i":2,"label":"review","reason":"Unclear"}]"#;
    assert_eq!(
        parse(array, 3),
        Ok(vec![(2, Label::Review, "Unclear".into())])
    );
}

#[test]
fn any_stray_text_or_bad_entry_fails_the_whole_answer() {
    for answer in [
        "Here are the labels:\n{\"i\":0,\"label\":\"safe\",\"reason\":\"x\"}",
        r#"{"i":3,"label":"safe","reason":"out of range"}"#,
        "{\"i\":0,\"label\":\"safe\",\"reason\":\"a\"}\n{\"i\":0,\"label\":\"keep\",\"reason\":\"b\"}",
        r#"{"i":0,"label":"delete","reason":"not a label"}"#,
        r#"{"i":-1,"label":"safe","reason":"negative"}"#,
        "",
    ] {
        let error = parse(answer, 3).unwrap_err();
        assert!(error.contains("nothing changed"), "{answer:?}: {error}");
    }
}

#[test]
fn reasons_are_cut_and_cleaned() {
    let long = "x".repeat(300);
    let answer = format!("{{\"i\":0,\"label\":\"review\",\"reason\":\"{long}\\u0007\"}}");
    let labels = parse(&answer, 1).unwrap();
    assert_eq!(labels[0].2.chars().count(), REASON_LIMIT);
    assert!(!labels[0].2.contains('\u{7}'));
}

#[test]
fn the_eval_fails_any_keep_fixture_labelled_safe_even_when_most_agree() {
    let mut labels: Vec<_> = FIXTURES
        .iter()
        .enumerate()
        .map(|(i, f)| (i, f.3, String::new()))
        .collect();
    assert!(passes(&score(&labels)));
    let keep = FIXTURES.iter().position(|f| f.3 == Label::Keep).unwrap();
    labels[keep].1 = Label::Safe;
    let s = score(&labels);
    assert_eq!(s.unsafe_safe, vec![keep]);
    assert!(!passes(&s), "one lost keep item fails the eval");
    // Labelling everything review is safe but useless, and fails on agreement.
    let all_review: Vec<_> = (0..FIXTURES.len())
        .map(|i| (i, Label::Review, String::new()))
        .collect();
    assert!(!passes(&score(&all_review)));
}
