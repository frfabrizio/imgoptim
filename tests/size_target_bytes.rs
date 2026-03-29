use imgoptim::cli::TargetSize;

#[test]
fn target_kb() {
    let kb = 250u64;
    let t = TargetSize::KiloBytes(kb);
    match t {
        TargetSize::KiloBytes(k) => assert_eq!(k * 1024, 256_000),
        TargetSize::Percent(_) => unreachable!(),
    }
}

#[test]
fn target_percent_math() {
    let ref_bytes = 1_000_000u64;
    let p = TargetSize::Percent(85);
    match p {
        TargetSize::Percent(pp) => assert_eq!((ref_bytes * u64::from(pp)) / 100, 850_000),
        TargetSize::KiloBytes(_) => unreachable!(),
    }
}
