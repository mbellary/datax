use datax_app_server_protocol::ChatSourceKind;
use datax_core::INTERACTIVE_SESSION_SOURCES;
use datax_protocol::protocol::SessionSource as CoreSessionSource;
use datax_protocol::protocol::SubAgentSource as CoreSubAgentSource;

pub(crate) fn compute_source_filters(
    source_kinds: Option<Vec<ChatSourceKind>>,
) -> (Vec<CoreSessionSource>, Option<Vec<ChatSourceKind>>) {
    let Some(source_kinds) = source_kinds else {
        return (INTERACTIVE_SESSION_SOURCES.to_vec(), None);
    };

    if source_kinds.is_empty() {
        return (INTERACTIVE_SESSION_SOURCES.to_vec(), None);
    }

    let requires_post_filter = source_kinds.iter().any(|kind| {
        matches!(
            kind,
            ChatSourceKind::Exec
                | ChatSourceKind::AppServer
                | ChatSourceKind::SubAgent
                | ChatSourceKind::SubAgentReview
                | ChatSourceKind::SubAgentCompact
                | ChatSourceKind::SubAgentThreadSpawn
                | ChatSourceKind::SubAgentOther
                | ChatSourceKind::Unknown
        )
    });

    if requires_post_filter {
        (Vec::new(), Some(source_kinds))
    } else {
        let interactive_sources = source_kinds
            .iter()
            .filter_map(|kind| match kind {
                ChatSourceKind::Cli => Some(CoreSessionSource::Cli),
                ChatSourceKind::VsCode => Some(CoreSessionSource::VSCode),
                ChatSourceKind::Exec
                | ChatSourceKind::AppServer
                | ChatSourceKind::SubAgent
                | ChatSourceKind::SubAgentReview
                | ChatSourceKind::SubAgentCompact
                | ChatSourceKind::SubAgentThreadSpawn
                | ChatSourceKind::SubAgentOther
                | ChatSourceKind::Unknown => None,
            })
            .collect::<Vec<_>>();
        (interactive_sources, Some(source_kinds))
    }
}

pub(crate) fn source_kind_matches(source: &CoreSessionSource, filter: &[ChatSourceKind]) -> bool {
    filter.iter().any(|kind| match kind {
        ChatSourceKind::Cli => matches!(source, CoreSessionSource::Cli),
        ChatSourceKind::VsCode => matches!(source, CoreSessionSource::VSCode),
        ChatSourceKind::Exec => matches!(source, CoreSessionSource::Exec),
        ChatSourceKind::AppServer => matches!(source, CoreSessionSource::Mcp),
        ChatSourceKind::SubAgent => matches!(source, CoreSessionSource::SubAgent(_)),
        ChatSourceKind::SubAgentReview => {
            matches!(
                source,
                CoreSessionSource::SubAgent(CoreSubAgentSource::Review)
            )
        }
        ChatSourceKind::SubAgentCompact => {
            matches!(
                source,
                CoreSessionSource::SubAgent(CoreSubAgentSource::Compact)
            )
        }
        ChatSourceKind::SubAgentThreadSpawn => matches!(
            source,
            CoreSessionSource::SubAgent(CoreSubAgentSource::ThreadSpawn { .. })
        ),
        ChatSourceKind::SubAgentOther => matches!(
            source,
            CoreSessionSource::SubAgent(CoreSubAgentSource::Other(_))
        ),
        ChatSourceKind::Unknown => matches!(source, CoreSessionSource::Unknown),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use datax_protocol::ThreadId;
    use pretty_assertions::assert_eq;
    use uuid::Uuid;

    #[test]
    fn compute_source_filters_defaults_to_interactive_sources() {
        let (allowed_sources, filter) = compute_source_filters(/*source_kinds*/ None);

        assert_eq!(allowed_sources, INTERACTIVE_SESSION_SOURCES.to_vec());
        assert_eq!(filter, None);
    }

    #[test]
    fn compute_source_filters_empty_means_interactive_sources() {
        let (allowed_sources, filter) = compute_source_filters(Some(Vec::new()));

        assert_eq!(allowed_sources, INTERACTIVE_SESSION_SOURCES.to_vec());
        assert_eq!(filter, None);
    }

    #[test]
    fn compute_source_filters_interactive_only_skips_post_filtering() {
        let source_kinds = vec![ChatSourceKind::Cli, ChatSourceKind::VsCode];
        let (allowed_sources, filter) = compute_source_filters(Some(source_kinds.clone()));

        assert_eq!(
            allowed_sources,
            vec![CoreSessionSource::Cli, CoreSessionSource::VSCode]
        );
        assert_eq!(filter, Some(source_kinds));
    }

    #[test]
    fn compute_source_filters_subagent_variant_requires_post_filtering() {
        let source_kinds = vec![ChatSourceKind::SubAgentReview];
        let (allowed_sources, filter) = compute_source_filters(Some(source_kinds.clone()));

        assert_eq!(allowed_sources, Vec::new());
        assert_eq!(filter, Some(source_kinds));
    }

    #[test]
    fn source_kind_matches_distinguishes_subagent_variants() {
        let parent_chat_id =
            ThreadId::from_string(&Uuid::new_v4().to_string()).expect("valid thread id");
        let review = CoreSessionSource::SubAgent(CoreSubAgentSource::Review);
        let spawn = CoreSessionSource::SubAgent(CoreSubAgentSource::ThreadSpawn {
            parent_chat_id,
            depth: 1,
            agent_path: None,
            agent_nickname: None,
            agent_role: None,
        });

        assert!(source_kind_matches(
            &review,
            &[ChatSourceKind::SubAgentReview]
        ));
        assert!(!source_kind_matches(
            &review,
            &[ChatSourceKind::SubAgentThreadSpawn]
        ));
        assert!(source_kind_matches(
            &spawn,
            &[ChatSourceKind::SubAgentThreadSpawn]
        ));
        assert!(!source_kind_matches(
            &spawn,
            &[ChatSourceKind::SubAgentReview]
        ));
    }
}
