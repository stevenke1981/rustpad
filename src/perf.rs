//! Small opt-in reproducible probes. No native keyboard/IME claims.
use eframe::egui;
use std::{path::Path, sync::OnceLock, time::Instant};
static START: OnceLock<Instant> = OnceLock::new();
pub fn mark_start() {
    let _ = START.set(Instant::now());
}
pub fn report_startup(path: &Path) {
    let elapsed = START.get().expect("probe start").elapsed().as_secs_f64() * 1000.;
    let json = format!("{{\"main_to_first_framebuffer_ms\":{elapsed:.4}}}\n");
    if let Err(error) = std::fs::write(path, json) {
        eprintln!("startup report: {error}");
    }
}
pub fn fixture(bytes: usize) -> String {
    let line = "/* 中文🙂 multiline\n still comment */\nfn sample() { let n = 42; let s = \"字串\"; println!(\"{n} {s}\"); }\n";
    let mut text = line.repeat(bytes / line.len() + 1);
    let mut end = bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text
}
pub fn benchmark(path: &Path, mut highlight: impl FnMut(&str) -> egui::text::LayoutJob) {
    let ctx = egui::Context::default();
    let mut rows = Vec::new();
    let _=ctx.run(Default::default(),|ctx|{for bytes in [16*1024,128*1024]{let text=fixture(bytes);let started=Instant::now();let _=highlight(&text);let first=started.elapsed().as_secs_f64()*1000.;let mut parse=Vec::new();let mut layout=Vec::new();let mut cached=Vec::new();let mut edit=Vec::new();for step in 0..5 {let mut changed=text.clone();let t=Instant::now();changed.insert_str(0,&format!("// edit {step}\n"));edit.push(t.elapsed().as_secs_f64()*1000.);let t=Instant::now();let job=highlight(&changed);parse.push(t.elapsed().as_secs_f64()*1000.);let t=Instant::now();ctx.fonts_mut(|fonts|fonts.layout_job(job.clone()));layout.push(t.elapsed().as_secs_f64()*1000.);let t=Instant::now();ctx.fonts_mut(|fonts|fonts.layout_job(job));cached.push(t.elapsed().as_secs_f64()*1000.);}let median=|mut data:Vec<f64>|{data.sort_by(f64::total_cmp);data[data.len()/2]};rows.push(format!("{{\"fixture_bytes\":{},\"iterations\":5,\"first_syntax_ms\":{first:.4},\"syntax_median_ms\":{:.4},\"uncached_layout_median_ms\":{:.4},\"cached_layout_median_ms\":{:.4},\"string_edit_median_ms\":{:.4}}}",text.len(),median(parse),median(layout),median(cached),median(edit)));}});
    let result = format!(
        "{{\"probe\":\"headless grammar+LayoutJob / default embedded fonts; not native keyboard latency\",\"samples\":[{}]}}\n",
        rows.join(",")
    );
    if let Err(error) = std::fs::write(path, result) {
        eprintln!("benchmark report: {error}");
    }
}
