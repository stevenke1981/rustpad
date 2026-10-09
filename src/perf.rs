//! Small opt-in reproducible probes. No native keyboard/IME claims.
use eframe::egui;
use std::{path::Path, sync::OnceLock, time::Instant};
static START: OnceLock<Instant> = OnceLock::new();
#[derive(Default)]
pub struct CaptureSchedule {
    frames: usize,
    requested: bool,
}
impl CaptureSchedule {
    pub fn due(&self, target: usize) -> bool {
        self.frames.saturating_add(1) >= target && !self.requested
    }
    pub fn poll(&mut self, target: usize, ready: bool) -> bool {
        self.frames = self.frames.saturating_add(1);
        if self.frames >= target && ready && !self.requested {
            self.requested = true;
            return true;
        }
        false
    }
}
pub fn profile_services(path: &Path) {
    let mut load = Vec::new();
    let mut clone = Vec::new();
    let mut compare = Vec::new();
    let mut decode = Vec::new();
    let text = fixture(128 * 1024);
    let snapshot = crate::session::Snapshot {
        tabs: (0..8)
            .map(|_| {
                let content = crate::core::Content {
                    text: text.clone(),
                    bom: true,
                    newline: crate::core::Newline::Crlf,
                };
                crate::session::Tab {
                    path: None,
                    saved: content.clone(),
                    content,
                    cursor: 0,
                    selected: (0, 0),
                }
            })
            .collect(),
        active: 0,
    };
    let bytes = crate::session::encode(&snapshot).expect("owned profile fixture");
    for _ in 0..5 {
        let timer = Instant::now();
        std::hint::black_box(crate::syntax::Engine::new());
        load.push(timer.elapsed().as_secs_f64() * 1000.);
        let timer = Instant::now();
        let copied = std::hint::black_box(snapshot.clone());
        clone.push(timer.elapsed().as_secs_f64() * 1000.);
        let timer = Instant::now();
        std::hint::black_box(copied == snapshot);
        compare.push(timer.elapsed().as_secs_f64() * 1000.);
        let timer = Instant::now();
        std::hint::black_box(crate::session::decode(&bytes).unwrap());
        decode.push(timer.elapsed().as_secs_f64() * 1000.);
    }
    let median = |mut values: Vec<f64>| {
        values.sort_by(f64::total_cmp);
        values[values.len() / 2]
    };
    let report = format!(
        "{{\"iterations\":5,\"tabs\":8,\"content_bytes_per_tab\":{},\"snapshot_bytes\":{},\"engine_load_median_ms\":{:.4},\"snapshot_clone_median_ms\":{:.4},\"snapshot_compare_median_ms\":{:.4},\"snapshot_decode_median_ms\":{:.4}}}\n",
        text.len(),
        bytes.len(),
        median(load),
        median(clone),
        median(compare),
        median(decode)
    );
    std::fs::write(path, report).expect("write profile report");
}
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delayed_restore_startup_capture_retries_and_requests_exactly_once() {
        for target in [1, 12] {
            let mut schedule = CaptureSchedule::default();
            for _ in 0..20 {
                assert!(!schedule.poll(target, false));
            }
            assert!(
                schedule.poll(target, true),
                "missed capture after async restore for delay {target}"
            );
            for _ in 0..20 {
                assert!(!schedule.poll(target, true));
            }
        }
    }
}
