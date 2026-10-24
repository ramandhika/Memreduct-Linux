use memreduct_core::meminfo::MemInfo;
use memreduct_core::mode::ReduceMode;
use memreduct_core::policy::{AutoReduceEngine, AutoReducePolicy, AutoReduceTrigger};
use memreduct_core::psi::{PsiLevel, PsiReport};
use memreduct_core::snapshot::MemorySnapshot;
use memreduct_core::zram::ZramStat;
use std::str::FromStr;

const SAMPLE_MEMINFO: &str = r#"
MemTotal:       16026132 kB
MemFree:          447144 kB
MemAvailable:    5713076 kB
Buffers:            1140 kB
Cached:          7366480 kB
SwapCached:            4 kB
Active:          9087068 kB
Inactive:        5486828 kB
Active(anon):    7789712 kB
Inactive(anon):  1341208 kB
Active(file):    1297356 kB
Inactive(file):  4145620 kB
Unevictable:      249968 kB
Mlocked:             304 kB
SwapTotal:      16025596 kB
SwapFree:       15839120 kB
Dirty:               108 kB
Writeback:             0 kB
AnonPages:       7824136 kB
Mapped:           603984 kB
Shmem:           1306784 kB
KReclaimable:     585984 kB
Slab:             840616 kB
SReclaimable:     585984 kB
SUnreclaim:       254632 kB
"#;

const SAMPLE_PSI: &str = r#"
some avg10=2.50 avg60=1.20 avg300=0.50 total=285587
full avg10=0.10 avg60=0.05 avg300=0.01 total=283984
"#;

const SAMPLE_ZRAM_STAT: &str = "187633664 40995041 43114496 0 43114496 7156 0 28 28";

#[test]
fn test_meminfo_parsing() {
    let info = MemInfo::parse_str(SAMPLE_MEMINFO, "/proc/meminfo").unwrap();

    assert_eq!(info.total_kb, 16026132);
    assert_eq!(info.free_kb, 447144);
    assert_eq!(info.available_kb, 5713076);
    assert_eq!(info.buffers_kb, 1140);
    assert_eq!(info.cached_kb, 7366480);
    assert_eq!(info.s_reclaimable_kb, 585984);
    assert_eq!(info.swap_total_kb, 16025596);
    assert_eq!(info.swap_free_kb, 15839120);

    // Calculated metrics
    let used_kb = info.used_kb();
    assert_eq!(used_kb, 16026132 - 5713076);
    assert!(info.used_percent() > 60.0 && info.used_percent() < 70.0);
    assert!(info.available_percent() > 30.0 && info.available_percent() < 40.0);
    assert_eq!(info.cache_and_buffers_kb(), 7366480 + 1140 + 585984);
    assert_eq!(info.swap_used_kb(), 16025596 - 15839120);
}

#[test]
fn test_psi_parsing() {
    let psi = PsiReport::parse_str(SAMPLE_PSI, "/proc/pressure/memory").unwrap();

    let some = psi.some.unwrap();
    assert_eq!(some.avg10, 2.50);
    assert_eq!(some.avg60, 1.20);
    assert_eq!(some.avg300, 0.50);
    assert_eq!(some.total_us, 285587);

    let full = psi.full.unwrap();
    assert_eq!(full.avg10, 0.10);
    assert_eq!(full.total_us, 283984);

    assert_eq!(psi.level(), PsiLevel::Low);
}

#[test]
fn test_psi_pressure_levels() {
    let critical_psi = "some avg10=65.0 avg60=40.0 avg300=20.0 total=1000\nfull avg10=25.0 avg60=10.0 avg300=5.0 total=500";
    let report = PsiReport::parse_str(critical_psi, "mock").unwrap();
    assert_eq!(report.level(), PsiLevel::Critical);

    let high_psi = "some avg10=35.0 avg60=20.0 avg300=10.0 total=1000\nfull avg10=1.0 avg60=0.0 avg300=0.0 total=500";
    let report_high = PsiReport::parse_str(high_psi, "mock").unwrap();
    assert_eq!(report_high.level(), PsiLevel::High);
}

#[test]
fn test_zram_stat_parsing() {
    let stat = ZramStat::parse_str(SAMPLE_ZRAM_STAT, "zram0", "mock").unwrap();
    assert_eq!(stat.orig_data_bytes, 187633664);
    assert_eq!(stat.compr_data_bytes, 40995041);
    assert_eq!(stat.mem_used_total_bytes, 43114496);

    let ratio = stat.compression_ratio();
    assert!(ratio > 4.5 && ratio < 4.6);
    assert_eq!(stat.saved_bytes(), 187633664 - 40995041);
}

#[test]
fn test_reduce_modes() {
    assert_eq!(
        ReduceMode::from_str("reclaim").unwrap(),
        ReduceMode::Reclaim
    );
    assert_eq!(
        ReduceMode::from_str("working-set").unwrap(),
        ReduceMode::Reclaim
    );
    assert_eq!(
        ReduceMode::from_str("pagecache").unwrap(),
        ReduceMode::PageCache
    );
    assert_eq!(
        ReduceMode::from_str("cache").unwrap(),
        ReduceMode::PageCache
    );
    assert_eq!(ReduceMode::from_str("slab").unwrap(), ReduceMode::Slab);
    assert_eq!(ReduceMode::from_str("all").unwrap(), ReduceMode::AllCaches);
    assert_eq!(
        ReduceMode::from_str("compact").unwrap(),
        ReduceMode::Compact
    );

    assert!(ReduceMode::from_str("unknown_mode").is_err());
}

#[test]
fn test_autoreduce_engine_logic() {
    let mut policy = AutoReducePolicy::default();
    policy.enabled_threshold = true;
    policy.threshold_used_percent = 60.0;
    policy.cooldown_seconds = 300;
    policy.default_mode = ReduceMode::Reclaim;

    let mut engine = AutoReduceEngine::new(policy);

    let meminfo = MemInfo::parse_str(SAMPLE_MEMINFO, "mock").unwrap();
    let snapshot = MemorySnapshot {
        timestamp_epoch_secs: 1000,
        meminfo,
        psi: None,
        zram: None,
    };

    // snapshot used_percent is ~64.3%, threshold is 60.0% -> should trigger
    let action = engine.check(&snapshot, 1000);
    assert!(action.is_some());
    let (mode, trigger) = action.unwrap();
    assert_eq!(mode, ReduceMode::Reclaim);
    match trigger {
        AutoReduceTrigger::Threshold(pct) => assert!(pct >= 60.0),
        _ => panic!("Expected threshold trigger"),
    }

    // Record reduce at t=1000
    engine.record_reduce(1000);

    // At t=1100 (elapsed 100s < 300s cooldown), must NOT trigger
    assert_eq!(engine.check(&snapshot, 1100), None);

    // At t=1301 (elapsed 301s > 300s cooldown), should trigger again
    assert!(engine.check(&snapshot, 1301).is_some());
}

#[test]
fn test_autoreduce_psi_trigger() {
    let mut policy = AutoReducePolicy::default();
    policy.enabled_psi = true;
    policy.psi_threshold_avg10 = 20.0;
    policy.cooldown_seconds = 60;

    let engine = AutoReduceEngine::new(policy);

    let meminfo = MemInfo::parse_str(SAMPLE_MEMINFO, "mock").unwrap();
    let psi_high = PsiReport::parse_str(
        "some avg10=28.5 avg60=15.0 avg300=5.0 total=100\nfull avg10=0.0 avg60=0.0 avg300=0.0 total=0",
        "mock",
    )
    .unwrap();

    let snapshot = MemorySnapshot {
        timestamp_epoch_secs: 2000,
        meminfo,
        psi: Some(psi_high),
        zram: None,
    };

    let action = engine.check(&snapshot, 2000);
    assert!(action.is_some());
    let (_, trigger) = action.unwrap();
    match trigger {
        AutoReduceTrigger::PsiPressure(avg10) => assert_eq!(avg10, 28.5),
        _ => panic!("Expected PSI pressure trigger"),
    }
}
