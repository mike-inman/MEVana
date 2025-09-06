use arb_engine::stats::{record_profit, record_depth_factor_bps, incr_signals_emitted, render_all};

#[test]
fn metrics_aliases_present() {
    // generate some samples
    record_profit(250);
    record_depth_factor_bps(9800);
    incr_signals_emitted();
    let rendered = render_all();
    assert!(rendered.contains("arb_profit_bps_bucket"), "missing alias profit histogram");
    assert!(rendered.contains("arb_depth_factor_bps_bucket"), "missing alias depth histogram");
    assert!(rendered.contains("arb_signals_emitted_total"), "missing signals counter");
}
