use jameskills_core::domain::{GuidanceAction, OfficialGuidanceSource, ToolId, ToolOperation};
use jameskills_infra::platform::{RenderedGuidanceAction, render_guidance_action};

#[test]
fn renderer_resolves_only_registered_sources_and_exact_command_pairs() {
    for (source, expected_id, expected_url) in [
        (
            OfficialGuidanceSource::GitInstall,
            "git-install",
            "https://git-scm.com/downloads",
        ),
        (
            OfficialGuidanceSource::Gitleaks,
            "gitleaks",
            "https://github.com/gitleaks/gitleaks",
        ),
        (
            OfficialGuidanceSource::ConventionalCommits,
            "conventional-commits",
            "https://www.conventionalcommits.org/en/v1.0.0/",
        ),
        (
            OfficialGuidanceSource::GithubCli,
            "github-cli",
            "https://cli.github.com/manual/gh_auth_login",
        ),
        (
            OfficialGuidanceSource::GithubRulesets,
            "github-rulesets",
            "https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets",
        ),
    ] {
        match render_guidance_action(&GuidanceAction::OpenOfficialUrl { source }) {
            RenderedGuidanceAction::OpenOfficialUrl { source_id, url } => {
                assert_eq!(source_id, expected_id);
                assert_eq!(url, expected_url);
            }
            _ => panic!("registered official source must render as an official URL"),
        }
    }
    assert!(matches!(
        render_guidance_action(&GuidanceAction::CopyApprovedCommand {
            tool_id: ToolId::Git,
            operation: ToolOperation::RepositoryRoot,
        }),
        RenderedGuidanceAction::CopyApprovedCommand {
            tool_id: ToolId::Git,
            operation: ToolOperation::RepositoryRoot,
            text: "git rev-parse --show-toplevel",
        }
    ));
    assert!(matches!(
        render_guidance_action(&GuidanceAction::CopyApprovedCommand {
            tool_id: ToolId::Gh,
            operation: ToolOperation::CheckRuns,
        }),
        RenderedGuidanceAction::Unsupported
    ));
}

#[test]
fn renderer_preserves_non_executable_user_interactions() {
    assert!(matches!(
        render_guidance_action(&GuidanceAction::ManualInstruction),
        RenderedGuidanceAction::ManualInstruction
    ));
    assert!(matches!(
        render_guidance_action(&GuidanceAction::SelectLocalPath {
            purpose: "repository-root".to_owned(),
        }),
        RenderedGuidanceAction::SelectLocalPath { purpose } if purpose == "repository-root"
    ));
    assert!(matches!(
        render_guidance_action(&GuidanceAction::AnswerChoice {
            choices: vec!["rust".to_owned(), "node".to_owned()],
        }),
        RenderedGuidanceAction::AnswerChoice { choices }
            if choices == ["rust", "node"]
    ));
    assert!(matches!(
        render_guidance_action(&GuidanceAction::Recheck),
        RenderedGuidanceAction::Recheck
    ));
}
