use agent_common::AgentCliAdapter;
use junie::JunieWrapper;
use std::process::Command;

#[test]
fn cli_help_and_version_are_compatible() {
    if std::env::var_os("FREQ_AI_LIVE_CLI_TESTS").is_none() {
        return;
    }

    let wrapper = JunieWrapper;
    for args in [wrapper.help_args(), wrapper.version_args()] {
        let status = Command::new(wrapper.binary())
            .args(args)
            .status()
            .expect("failed to spawn provider binary");
        assert!(status.success());
    }
}
