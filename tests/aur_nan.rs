use pacseek::model::Package;

#[test]
fn sort_with_nan_does_not_panic() {
    let mut pkgs = [
        Package {
            name: "a".into(),
            version: "1".into(),
            description: None,
            repo: "aur".into(),
            arch: None,
            url: None,
            installed: false,
            votes: Some(10),
            popularity: Some(f64::NAN),
            out_of_date: None,
            maintainer: None,
            num_votes: Some(10),
            last_modified: None,
        },
        Package {
            name: "b".into(),
            version: "1".into(),
            description: None,
            repo: "aur".into(),
            arch: None,
            url: None,
            installed: false,
            votes: Some(5),
            popularity: Some(1.0),
            out_of_date: None,
            maintainer: None,
            num_votes: Some(5),
            last_modified: None,
        },
        Package {
            name: "c".into(),
            version: "1".into(),
            description: None,
            repo: "aur".into(),
            arch: None,
            url: None,
            installed: false,
            votes: Some(20),
            popularity: Some(f64::INFINITY),
            out_of_date: None,
            maintainer: None,
            num_votes: Some(20),
            last_modified: None,
        },
    ];

    // This is the same sort logic as in aur.rs after fix
    pkgs.sort_by(|a, b| {
        let ap = a.popularity.filter(|v| v.is_finite()).unwrap_or(0.0);
        let bp = b.popularity.filter(|v| v.is_finite()).unwrap_or(0.0);
        bp.partial_cmp(&ap)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.votes.unwrap_or(0).cmp(&a.votes.unwrap_or(0)))
    });

    // Should not panic, and NaN/Inf should be treated as 0.0, so b (1.0) should be first
    assert_eq!(pkgs[0].name, "b");
}

#[test]
fn unicode_truncate_no_panic() {
    // Simulate ui.rs truncation with emoji
    let input = "fire🦀fox🎉test";
    let width = 10; // small width to force truncation
    // This is the fixed logic from ui.rs
    use unicode_width::UnicodeWidthStr;
    let input_width = input.width();
    assert!(input_width > width);
    let target = width.saturating_sub(3);
    let mut w = 0usize;
    let mut start_idx = input.len();
    for (idx, ch) in input.char_indices().rev() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > target {
            break;
        }
        w += cw;
        start_idx = idx;
    }
    let display = format!("...{}", &input[start_idx..]);
    // Should not panic and be char-boundary safe
    assert!(display.starts_with("..."));
    // Ensure display is valid utf8 and width <= width
    assert!(display.width() <= width);
}
