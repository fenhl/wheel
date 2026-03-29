//! This module is based on <https://github.com/NixOS/nixpkgs/pull/203228>.
//! The current version is based on the state of that PR as of commit 7396c59.

use {
    std::{
        collections::HashMap,
        process::Stdio,
    },
    futures::{
        future::FutureExt as _,
        stream::{
            self,
            Stream,
        },
    },
    serde::Deserialize,
    tokio::{
        io::{
            AsyncBufReadExt as _,
            BufReader,
            Lines,
        },
        process::{
            Child,
            ChildStderr,
            Command,
        },
        select,
        signal::unix::{
            Signal,
            SignalKind,
            signal,
        },
    },
    crate::{
        Result,
        traits::{
            IoResultExt as _,
            SyncCommandOutputExt as _,
        },
    },
};

const ACT_COPY_PATH: u8 = 100;
const ACT_COPY_PATHS: u8 = 103;
const ACT_BUILDS: u8 = 104;
const RES_PROGRESS: u8 = 105;

/// Status updates yielded by [`rebuild`].
pub enum RebuildMessage {
    /// Update the completion percentage of the rebuild. Value is between `0.0` and `1.0` inclusive.
    Progress(f64),
    /// A line of text that should be displayed to the user.
    Log(String),
    /// The process has received `SIGTERM` and should exit.
    ///
    /// This can happen if your process is running as a systemd service which needs to be restarted due to the rebuild.
    Sigterm,
}

/// Triggers a NixOS rebuild.
pub fn rebuild() -> impl Stream<Item = Result<RebuildMessage>> {
    #[derive(PartialEq, Eq)]
    enum ProgressKind {
        Builds,
        Copies,
    }

    enum State {
        Init,
        Normal {
            sigterm: Signal,
            stderr: Lines<BufReader<ChildStderr>>,
            child: Child,
            pending_text: Option<String>,
            per_act_progress: HashMap<(u8, u64), (ProgressKind, usize, usize)>,
            copy_bytes: HashMap<u64, (usize, usize)>,
            activities: HashMap<u64, u8>,
            fraction: f64,
            floor: f64,
            floor_done: usize,
        },
        Done,
    }

    stream::try_unfold(State::Init, |mut state| async {
        Ok(Some(loop {
            break (match &mut state {
                State::Init => {
                    let mut child = Command::new("/run/wrappers/bin/sudo")
                        .arg("--non-interactive")
                        .arg("/run/current-system/sw/bin/nixos-rebuild")
                        .arg("switch")
                        .arg("--recreate-lock-file")
                        .arg("--refresh")
                        .arg("--no-write-lock-file")
                        .arg("--flake=git+ssh://fenhl@fenhl.net/opt/git/localhost/dev/dev.git")
                        .arg("--log-format")
                        .arg("internal-json")
                        .stderr(Stdio::piped())
                        .spawn().at_command("nixos-rebuild")?;
                    state = State::Normal {
                        sigterm: signal(SignalKind::terminate()).at_unknown()?,
                        stderr: BufReader::new(child.stderr.take().expect("configured above")).lines(),
                        child,
                        pending_text: None,
                        per_act_progress: HashMap::default(),
                        copy_bytes: HashMap::default(),
                        activities: HashMap::default(),
                        fraction: 0.0,
                        floor: 0.0,
                        floor_done: 0,
                    };
                    continue
                }
                State::Normal {
                    sigterm,
                    stderr,
                    child,
                    pending_text,
                    per_act_progress,
                    copy_bytes,
                    activities,
                    fraction,
                    floor,
                    floor_done,
                } => if let Some(text) = pending_text.take() {
                    RebuildMessage::Log(text)
                } else {
                    select! {
                        _ = sigterm.recv() => {
                            state = State::Done;
                            RebuildMessage::Sigterm
                        }
                        Some(res) = stderr.next_line().map(Result::transpose) => {
                            let line = res.at_command("nixos-rebuild")?;
                            if let Some(line) = line.strip_prefix("@nix ") {
                                #[derive(Deserialize)]
                                #[serde(tag = "action", rename_all = "lowercase")]
                                enum Action {
                                    Start {
                                        id: u64,
                                        #[serde(rename = "type", default)]
                                        kind: u8,
                                        #[serde(default)]
                                        text: String,
                                        #[serde(default)]
                                        level: i32,
                                    },
                                    Stop {
                                        id: u64,
                                    },
                                    Result {
                                        id: u64,
                                        #[serde(rename = "type", default)]
                                        res_kind: u8,
                                        #[serde(default)]
                                        fields: Vec<usize>,
                                    },
                                    Msg {
                                        #[serde(default)]
                                        level: i32,
                                        #[serde(default)]
                                        msg: String,
                                    },
                                }

                                let text = match serde_json::from_str(line).at_command("nixos-rebuild")? {
                                    Action::Start { id, kind, text, level } => {
                                        activities.insert(id, kind);
                                        if level <= 3 && !text.is_empty() {
                                            Some(text)
                                        } else {
                                            None
                                        }
                                    }
                                    Action::Stop { id } => {
                                        activities.remove(&id);
                                        copy_bytes.remove(&id);
                                        None
                                    }
                                    Action::Result { id, res_kind, fields } => {
                                        let act_kind = activities.get(&id).copied().unwrap_or_default();
                                        if res_kind == RES_PROGRESS && let &[done, expected, ..] = &*fields {
                                            match act_kind {
                                                ACT_BUILDS => { per_act_progress.insert((act_kind, id), (ProgressKind::Builds, done, expected)); }
                                                ACT_COPY_PATHS => { per_act_progress.insert((act_kind, id), (ProgressKind::Copies, done, expected)); }
                                                ACT_COPY_PATH => { copy_bytes.insert(id, (done, expected.max(1))); }
                                                _ => {}
                                            }                                        
                                        }
                                        None
                                    }
                                    Action::Msg { level, msg } => if level <= 1 && !msg.is_empty() {
                                        Some(msg)
                                    } else {
                                        None
                                    },
                                };
                                let builds_done = per_act_progress.values().filter(|(kind, _, _)| *kind == ProgressKind::Builds).map(|(_, done, _)| done).sum::<usize>();
                                let builds_expected = per_act_progress.values().filter(|(kind, _, _)| *kind == ProgressKind::Builds).map(|(_, _, expected)| expected).sum::<usize>();
                                let copies_done = per_act_progress.values().filter(|(kind, _, _)| *kind == ProgressKind::Copies).map(|(_, done, _)| done).sum::<usize>();
                                let copies_expected = per_act_progress.values().filter(|(kind, _, _)| *kind == ProgressKind::Copies).map(|(_, _, expected)| expected).sum::<usize>();
                                let count_total = builds_expected + copies_expected;
                                let count_done = builds_done + copies_done;
                                let prev_fraction = *fraction;
                                if count_total > 0 {
                                    // Prevents the channel copy (1 path) from dominating the entire progress bar before the main build starts.
                                    let effective_total = count_total.max(3);
                                    let count_frac = count_done as f64 / effective_total as f64;
                                    // large single-path copies move the bar.
                                    let total_bytes = copy_bytes.values().map(|(_, t)| t).sum::<usize>();
                                    let done_bytes = copy_bytes.values().map(|(d, _)| d).sum::<usize>();
                                    *fraction = if total_bytes > 0 {
                                        let byte_sub = done_bytes as f64 / total_bytes as f64;
                                        let step = copy_bytes.len() as f64 / effective_total as f64;
                                        count_frac + byte_sub * step
                                    } else {
                                        count_frac
                                    }.clamp(0.0, 1.0);
                                    // When the denominator grows (e.g. main build discovers hundreds of new paths), remap remaining work into remaining bar space.
                                    if *fraction > *floor {
                                        *floor = *fraction;
                                        *floor_done = count_done;
                                    } else {
                                        let new_items = isize::try_from(count_done).expect("integer overflow").strict_sub_unsigned(*floor_done);
                                        let remaining = isize::try_from(effective_total).expect("integer overflow").strict_sub_unsigned(*floor_done);
                                        *fraction = if remaining > 0 && new_items >= 0 {
                                            *floor + (new_items as f64 / remaining as f64) * (1.0 - *floor)
                                        } else {
                                            *floor
                                        }.clamp(0.0, 1.0);
                                        if *fraction > *floor {
                                            *floor = *fraction;
                                            *floor_done = count_done;
                                        }
                                    }
                                }
                                if *fraction != prev_fraction {
                                    *pending_text = text;
                                    RebuildMessage::Progress(*fraction)
                                } else if let Some(text) = text {
                                    RebuildMessage::Log(text)
                                } else {
                                    continue
                                }
                            } else {
                                RebuildMessage::Log(line)
                            }
                        }
                        res = child.wait() => {
                            res.at_command("nixos-rebuild")?.check("nixos-rebuild")?;
                            return Ok(None)
                        }
                    }
                },
                State::Done => return Ok(None),
            }, state)
        }))
    })
}

/*
#[tokio::test]
async fn test_rebuild() -> Result {
    use {
        std::pin::pin,
        futures::stream::TryStreamExt as _,
    };

    let mut updates = pin!(rebuild());
    while let Some(update) = updates.try_next().await? {
        match update {
            RebuildMessage::Progress(progress) => println!("progress: {}%\r", (progress * 1000.0).floor() / 10.0),
            RebuildMessage::Log(msg) => println!("log: {}\r", msg.replace("\n", "\r\n")),
            RebuildMessage::Sigterm => println!("SIGTERM\r"),
        }
    }
    Ok(())
}
*/
