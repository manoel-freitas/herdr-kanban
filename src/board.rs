//! Pure board model: issues + agents -> columns. No IO.

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Label {
    pub name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct User {
    pub login: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    /// "OPEN" | "CLOSED"
    pub state: String,
    pub labels: Vec<Label>,
    pub assignees: Vec<User>,
    pub url: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Agent {
    pub agent: String,
    pub agent_status: String,
    pub pane_id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub terminal_title_stripped: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Column {
    Todo,
    InProgress,
    Review,
    Done,
}

impl Column {
    pub const ALL: [Column; 4] = [Column::Todo, Column::InProgress, Column::Review, Column::Done];

    pub fn title(self) -> &'static str {
        match self {
            Column::Todo => "Todo",
            Column::InProgress => "In Progress",
            Column::Review => "Review",
            Column::Done => "Done",
        }
    }

    /// Done is the closed state, not a label.
    pub fn label(self) -> Option<&'static str> {
        match self {
            Column::Todo => Some("status:todo"),
            Column::InProgress => Some("status:in-progress"),
            Column::Review => Some("status:review"),
            Column::Done => None,
        }
    }
}

pub struct Card {
    pub issue: Issue,
    pub agents: Vec<Agent>,
}

pub struct Board {
    pub columns: [Vec<Card>; 4],
    pub unlinked: Vec<Agent>,
}

pub fn column_of(i: &Issue) -> Column {
    if i.state == "CLOSED" {
        return Column::Done;
    }
    let has = |l: &str| i.labels.iter().any(|x| x.name == l);
    if has("status:review") {
        Column::Review
    } else if has("status:in-progress") {
        Column::InProgress
    } else {
        Column::Todo
    }
}

pub fn issue_refs(a: &Agent) -> Vec<u64> {
    let mut out = Vec::new();
    if let Some(n) = a
        .cwd
        .rsplit('/')
        .next()
        .and_then(|s| s.strip_prefix("issue-"))
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|s| s.parse().ok())
    {
        out.push(n);
    }
    let t = a.terminal_title_stripped.as_str();
    for (i, _) in t.match_indices('#') {
        let digits = &t[i + 1..];
        let end = digits.bytes().position(|b| !b.is_ascii_digit()).unwrap_or(digits.len());
        if end > 0 {
            if let Ok(n) = digits[..end].parse() {
                out.push(n);
            }
        }
    }
    let mut seen = Vec::new();
    out.retain(|n| {
        let new = !seen.contains(n);
        seen.push(*n);
        new
    });
    out
}

pub fn build(issues: Vec<Issue>, agents: Vec<Agent>) -> Board {
    let refs: Vec<Vec<u64>> = agents.iter().map(issue_refs).collect();
    let mut columns: [Vec<Card>; 4] = Default::default();
    for issue in issues {
        let linked = agents
            .iter()
            .zip(&refs)
            .filter(|(_, r)| r.contains(&issue.number))
            .map(|(a, _)| a.clone())
            .collect();
        columns[column_of(&issue) as usize].push(Card { issue, agents: linked });
    }
    let all: Vec<u64> = columns.iter().flatten().map(|c| c.issue.number).collect();
    let unlinked = agents
        .into_iter()
        .zip(refs)
        .filter(|(_, r)| !r.iter().any(|n| all.contains(n)))
        .map(|(a, _)| a)
        .collect();
    Board { columns, unlinked }
}

pub fn owner_line(c: &Card) -> String {
    if !c.agents.is_empty() {
        return c
            .agents
            .iter()
            .map(|a| format!("🤖 {} {} {}", a.agent, a.pane_id, a.agent_status))
            .collect::<Vec<_>>()
            .join("  ");
    }
    if !c.issue.assignees.is_empty() {
        return c
            .issue
            .assignees
            .iter()
            .map(|u| format!("@{}", u.login))
            .collect::<Vec<_>>()
            .join(" ");
    }
    if let Some(name) = c.issue.labels.iter().find_map(|l| l.name.strip_prefix("agent:")) {
        return format!("🤖 {name} (offline)");
    }
    "unassigned".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(n: u64, state: &str, labels: &[&str]) -> Issue {
        Issue {
            number: n,
            title: format!("t{n}"),
            state: state.into(),
            labels: labels.iter().map(|l| Label { name: (*l).into() }).collect(),
            assignees: vec![],
            url: String::new(),
        }
    }

    fn agent(cwd: &str, title: &str) -> Agent {
        Agent {
            agent: "omp".into(),
            agent_status: "idle".into(),
            pane_id: "w1:p1".into(),
            workspace_id: "w1".into(),
            cwd: cwd.into(),
            terminal_title_stripped: title.into(),
        }
    }

    #[test]
    fn column_of_cases() {
        assert_eq!(column_of(&issue(1, "CLOSED", &["status:review"])), Column::Done);
        assert_eq!(
            column_of(&issue(1, "OPEN", &["status:review", "status:in-progress"])),
            Column::Review
        );
        assert_eq!(column_of(&issue(1, "OPEN", &["status:in-progress"])), Column::InProgress);
        assert_eq!(column_of(&issue(1, "OPEN", &["bug"])), Column::Todo);
    }

    #[test]
    fn issue_refs_cases() {
        assert_eq!(issue_refs(&agent("/x/.worktrees/issue-33", "")), vec![33]);
        assert_eq!(issue_refs(&agent("/x/issue-3a", "")), Vec::<u64>::new());
        assert_eq!(issue_refs(&agent("/x", "fix #12 and #7")), vec![12, 7]);
        assert_eq!(issue_refs(&agent("/x", "#")), Vec::<u64>::new());
    }

    #[test]
    fn build_links_and_unlinked() {
        let b = build(
            vec![issue(5, "OPEN", &[])],
            vec![agent("/x/.worktrees/issue-5", ""), agent("/y", "nothing")],
        );
        assert_eq!(b.columns[Column::Todo as usize][0].agents.len(), 1);
        assert_eq!(b.unlinked.len(), 1);
        assert_eq!(b.unlinked[0].cwd, "/y");
    }
}
