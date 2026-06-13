use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub struct FlamegraphSettings {
    pub output_dir: Option<PathBuf>,
    pub frequency_hz: i32,
}

#[derive(Clone, Debug)]
pub enum FlamegraphEvent {
    Written { path: PathBuf, elapsed_us: u128 },
    Failed { reason: String },
    Skipped { reason: &'static str },
}

impl FlamegraphSettings {
    pub fn from_parts(output_dir: Option<PathBuf>, frequency_hz: i32) -> Self {
        Self {
            output_dir,
            frequency_hz,
        }
    }

    pub fn output_dir(&self) -> Option<&Path> {
        self.output_dir.as_deref()
    }
}

pub fn instrumentation_available() -> bool {
    cfg!(not(feature = "no_instrument"))
}

#[cfg(not(feature = "no_instrument"))]
pub fn profile_solver_call<T>(
    settings: &FlamegraphSettings,
    sample: usize,
    approach: &str,
    run: impl FnOnce() -> T,
) -> (T, Option<FlamegraphEvent>) {
    use pprof::ProfilerGuard;
    use std::fs::{create_dir_all, File};
    use std::time::Instant;

    let Some(output_dir) = settings.output_dir() else {
        return (run(), None);
    };
    if settings.frequency_hz <= 0 {
        return (
            run(),
            Some(FlamegraphEvent::Failed {
                reason: format!("invalid flamegraph frequency {}", settings.frequency_hz),
            }),
        );
    }
    if let Err(error) = create_dir_all(output_dir) {
        return (
            run(),
            Some(FlamegraphEvent::Failed {
                reason: format!("failed to create flamegraph dir {output_dir:?}: {error}"),
            }),
        );
    }

    let guard = match ProfilerGuard::new(settings.frequency_hz) {
        Ok(guard) => guard,
        Err(error) => {
            return (
                run(),
                Some(FlamegraphEvent::Failed {
                    reason: format!("failed to start profiler guard: {error}"),
                }),
            );
        }
    };

    let started = Instant::now();
    let output = run();
    let report = match guard.report().build() {
        Ok(report) => report,
        Err(error) => {
            return (
                output,
                Some(FlamegraphEvent::Failed {
                    reason: format!("failed to build profiler report: {error}"),
                }),
            );
        }
    };

    let filename = format!(
        "sample_{sample:04}_{}.svg",
        sanitize_name_fragment(approach)
    );
    let path = output_dir.join(filename);
    let file = match File::create(&path) {
        Ok(file) => file,
        Err(error) => {
            return (
                output,
                Some(FlamegraphEvent::Failed {
                    reason: format!("failed to create flamegraph file {path:?}: {error}"),
                }),
            );
        }
    };
    if let Err(error) = report.flamegraph(file) {
        return (
            output,
            Some(FlamegraphEvent::Failed {
                reason: format!("failed to write flamegraph {path:?}: {error}"),
            }),
        );
    }

    (
        output,
        Some(FlamegraphEvent::Written {
            path,
            elapsed_us: started.elapsed().as_micros(),
        }),
    )
}

#[cfg(feature = "no_instrument")]
pub fn profile_solver_call<T>(
    settings: &FlamegraphSettings,
    _sample: usize,
    _approach: &str,
    run: impl FnOnce() -> T,
) -> (T, Option<FlamegraphEvent>) {
    let output = run();
    let event = settings
        .output_dir()
        .is_some()
        .then_some(FlamegraphEvent::Skipped {
            reason: "compiled_with_no_instrument",
        });
    (output, event)
}

#[cfg(not(feature = "no_instrument"))]
fn sanitize_name_fragment(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "unnamed".to_string()
    } else {
        sanitized
    }
}
