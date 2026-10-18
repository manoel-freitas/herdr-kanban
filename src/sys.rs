//! herdr + gh IO.

use crate::board::{Agent, Column, Issue, column_of};
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn run(cmd: &str, args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
    let mut c = Command::new(cmd);
    c.args(args).stdin(Stdio::null());
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    let out = c.output().map_err(|e| format!("{cmd}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{cmd} {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn herdr_bin() -> String {
    std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".into())
}

fn herdr(args: &[&str]) -> Result<String, String> {
    run(&herdr_bin(), args, None)
}

pub struct Ctx {
    /// owner/name
    pub repo: String,
    pub repo_key: String,
}

#[derive(Deserialize)]
struct Worktree {
    #[serde(default)]
    checkout_path: String,
    #[serde(default)]
    repo_key: String,
}

#[derive(Deserialize)]
struct Workspace {
    workspace_id: String,
    worktree: Option<Worktree>,
}

fn workspaces() -> Result<Vec<Workspace>, String> {
    let out = herdr(&["workspace", "list"])?;
    let v: Value = serde_json::from_str(&out).map_err(|e| format!("herdr: unexpected JSON: {e}"))?;
    serde_json::from_value(v["result"]["workspaces"].clone())
        .map_err(|e| format!("herdr: unexpected JSON: {e}"))
}

pub fn resolve_ctx() -> Result<Ctx, String> {
    let dir: PathBuf = match std::env::var("HERDR_WORKSPACE_ID") {
        Ok(id) => workspaces()?
            .into_iter()
            .find(|w| w.workspace_id == id)
            .and_then(|w| w.worktree)
            .map(|wt| PathBuf::from(wt.checkout_path))
            .ok_or("workspace has no git worktree")?,
        Err(_) => std::env::current_dir().map_err(|e| e.to_string())?,
    };
    let repo_key = run(
        "git",
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        Some(&dir),
    )
    .map_err(|e| format!("{}: not a git repo: {e}", dir.display()))?
    .trim()
    .to_string();
    let repo = match std::env::var("HERDR_KANBAN_REPO") {
        Ok(r) => r,
        Err(_) => run(
            "gh",
            &["repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"],
            Some(&dir),
        )
        .map(|s| s.trim().to_string())
        .map_err(|e| {
            format!(
                "no GitHub repo for {}: {e}. Set HERDR_KANBAN_REPO=owner/name",
                dir.display()
            )
        })?,
    };
    Ok(Ctx { repo, repo_key })
}

pub fn fetch_issues(repo: &str) -> Result<Vec<Issue>, String> {
    let mut all = Vec::new();
    for (state, limit) in [("open", "200"), ("closed", "20")] {
        let out = run(
            "gh",
            &[
                "issue", "list", "--repo", repo, "--state", state, "--limit", limit, "--json",
                "number,title,state,labels,assignees,url",
            ],
            None,
        )?;
        let mut v: Vec<Issue> =
            serde_json::from_str(&out).map_err(|e| format!("gh: unexpected JSON: {e}"))?;
        all.append(&mut v);
    }
    Ok(all)
}

pub fn fetch_agents(repo_key: &str) -> Result<Vec<Agent>, String> {
    let ids: Vec<String> = workspaces()?
        .into_iter()
        .filter(|w| w.worktree.as_ref().is_some_and(|t| t.repo_key == repo_key))
        .map(|w| w.workspace_id)
        .collect();
    let out = herdr(&["agent", "list"])?;
    let v: Value = serde_json::from_str(&out).map_err(|e| format!("herdr: unexpected JSON: {e}"))?;
    let agents: Vec<Agent> = serde_json::from_value(v["result"]["agents"].clone())
        .map_err(|e| format!("herdr: unexpected JSON: {e}"))?;
    Ok(agents.into_iter().filter(|a| ids.contains(&a.workspace_id)).collect())
}

pub fn move_issue(repo: &str, issue: &Issue, to: Column) -> Result<(), String> {
    let from = column_of(issue);
    if from == to {
        return Ok(());
    }
    let n = issue.number.to_string();
    let old: Vec<&str> = issue
        .labels
        .iter()
        .map(|l| l.name.as_str())
        .filter(|l| l.starts_with("status:"))
        .collect();
    let Some(target) = to.label() else {
        if !old.is_empty() {
            run("gh", &["issue", "edit", &n, "--repo", repo, "--remove-label", &old.join(",")], None)?;
        }
        return run("gh", &["issue", "close", &n, "--repo", repo], None).map(drop);
    };
    if from == Column::Done {
        run("gh", &["issue", "reopen", &n, "--repo", repo], None)?;
    }
    let color = match to {
        Column::Todo => "ededed",
        Column::InProgress => "fbca04",
        _ => "0e8a16",
    };
    // Err ignored: label may already exist. No --force, keeps user colors.
    let _ = run("gh", &["label", "create", target, "--repo", repo, "--color", color], None);
    let stale: Vec<&str> = old.into_iter().filter(|l| *l != target).collect();
    let stale = stale.join(",");
    let mut args = vec!["issue", "edit", &n, "--repo", repo, "--add-label", target];
    if !stale.is_empty() {
        args.extend(["--remove-label", &stale]);
    }
    run("gh", &args, None).map(drop)
}

pub fn focus_agent(pane_id: &str) -> Result<(), String> {
    herdr(&["agent", "focus", pane_id]).map(drop)
}

pub fn open_url(url: &str) -> Result<(), String> {
    let bin = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    Command::new(bin)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
        .map_err(|e| format!("{bin}: {e}"))
}
