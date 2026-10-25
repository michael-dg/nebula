//! SUMMARY — one sentence on what a session is doing, kept on its row so
//! its card says where the work stands even when the last prompt is "ok"
//! or "continue".
//!
//! The session's own agent writes it: [`CLAUDE_SUMMARY_GUIDANCE`] rides
//! nebula's launch guidance and asks the agent to run `nebula summary
//! "<sentence>"` once it understands the task and again whenever the goal
//! changes. The daemon makes no model call of its own. The sentence is
//! tidied to one line of at most [`MAX_SUMMARY_CHARS`], stored on the row
//! and reaches every TUI as an ordinary upsert. A new conversation in the
//! same CLI (`/clear`, `/new`) empties it along with RECENT PROMPTS.

use std::sync::Arc;

use anyhow::{bail, Result};
use nebula_core::AgentId;

use crate::prompt_history::one_line;
use crate::registry::Daemon;

/// The longest summary the store keeps: one sentence, about one card line.
pub const MAX_SUMMARY_CHARS: usize = 80;

/// Appended to the agent's system prompt (see `CLAUDE_WORKTREE_GUIDANCE`)
/// so the agent keeps its card's SUMMARY current.
pub const CLAUDE_SUMMARY_GUIDANCE: &str = "[nebula] This session's card in nebula shows a \
one-sentence summary of what you are doing, which you write yourself. Once you understand the \
user's task, run this shell command:\n\n  nebula summary \"<sentence>\"\n\nwhere <sentence> \
says, in at most 80 characters, the outcome the user will get (example: nebula summary \
\"Login redirects back to the page the user came from\") — not the steps or files involved. Run \
it again only when the goal changes, not for each step. Don't mention it to the user.";

impl Daemon {
    /// `nebula summary` from inside the session: replace the row's SUMMARY
    /// with `raw` as one line and push the row to every subscriber.
    pub fn set_summary(self: &Arc<Self>, id: &AgentId, raw: &str) -> Result<()> {
        let summary = one_line(raw, MAX_SUMMARY_CHARS);
        if summary.is_empty() {
            bail!("summary is empty");
        }
        if !self.store.set_summary(id, &summary)? {
            bail!("agent not found");
        }
        self.try_broadcast_agent(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt_history::tests::daemon_with_one_agent;
    use nebula_core::{Entity, ServerEvent};

    /// The sentence lands on the row as one tidy line and reaches
    /// subscribers as the row's upsert; a later call replaces it.
    #[tokio::test]
    async fn set_summary_stores_one_line_and_upserts_the_row() {
        let (daemon, id) = daemon_with_one_agent();
        let mut events = daemon.events.subscribe();

        daemon
            .set_summary(&id, "  Login redirects\n back\u{1b}[31m home  ")
            .unwrap();
        match events.try_recv() {
            Ok(ServerEvent::EntityUpserted {
                entity: Entity::Agent(a),
            }) => assert_eq!(a.summary.as_deref(), Some("Login redirects back [31m home")),
            other => panic!("expected the row's upsert, got {other:?}"),
        }

        daemon
            .set_summary(&id, "Logout clears the session")
            .unwrap();
        let stored = daemon.store.get_agent(&id).unwrap().unwrap();
        assert_eq!(stored.summary.as_deref(), Some("Logout clears the session"));
    }

    /// A longer sentence is clipped with an ellipsis at the cap.
    #[tokio::test]
    async fn set_summary_clips_past_the_cap() {
        let (daemon, id) = daemon_with_one_agent();
        daemon.set_summary(&id, &"word ".repeat(40)).unwrap();
        let summary = daemon
            .store
            .get_agent(&id)
            .unwrap()
            .unwrap()
            .summary
            .unwrap();
        assert_eq!(summary.chars().count(), MAX_SUMMARY_CHARS);
        assert!(summary.ends_with('…'));
    }

    /// A blank sentence and an unknown row are refused, and nothing is
    /// broadcast or stored.
    #[tokio::test]
    async fn set_summary_refuses_a_blank_sentence_and_an_unknown_row() {
        let (daemon, id) = daemon_with_one_agent();
        let mut events = daemon.events.subscribe();

        let err = daemon.set_summary(&id, " \u{7}\n ").unwrap_err();
        assert_eq!(err.to_string(), "summary is empty");
        let err = daemon
            .set_summary(&AgentId("ghost".into()), "Something")
            .unwrap_err();
        assert_eq!(err.to_string(), "agent not found");

        assert!(events.try_recv().is_err());
        assert_eq!(daemon.store.get_agent(&id).unwrap().unwrap().summary, None);
    }
}
