use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// RAII guard that kills the tmux session on drop.
pub struct Session {
    pub name: String,
}

impl Session {
    pub fn new() -> Self {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let name = format!("test-{}-{}", std::process::id(), id);
        Session { name }
    }

    fn bin_path() -> String {
        // Find the agent-terminal binary relative to the test binary
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // remove test binary name
        path.pop(); // remove deps/
        path.push("agent-terminal");
        path.to_string_lossy().to_string()
    }

    pub fn fixture_path(name: &str) -> String {
        let mut path = std::env::current_exe().unwrap();
        path.pop();
        path.pop();
        path.push(format!("fixture-{}", name));
        path.to_string_lossy().to_string()
    }

    /// Run an agent-terminal command targeting this session.
    pub fn run(&self, args: &[&str]) -> Output {
        let bin = Self::bin_path();
        let mut cmd = Command::new(&bin);
        cmd.args(args);

        // Add --session flag if not already present and not a command that doesn't take it
        let needs_session = !args.contains(&"--session")
            && !args.is_empty()
            && args[0] != "list"
            && args[0] != "doctor"
            && args[0] != "init";

        if needs_session {
            cmd.arg("--session").arg(&self.name);
        }

        cmd.output().expect("failed to run agent-terminal")
    }

    /// Run and assert success, returning stdout.
    pub fn run_ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "Command {:?} failed (exit {})\nstdout: {}\nstderr: {}",
            args,
            out.status.code().unwrap_or(-1),
            stdout,
            stderr
        );
        stdout.to_string()
    }

    /// Run and expect failure, returning stderr.
    pub fn run_fail(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(
            !out.status.success(),
            "Command {:?} should have failed but succeeded\nstdout: {}",
            args,
            String::from_utf8_lossy(&out.stdout)
        );
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        format!("{}{}", stdout, stderr)
    }

    /// Open a fixture in this session.
    pub fn open_fixture(&self, fixture_name: &str) {
        let path = Self::fixture_path(fixture_name);
        self.run_ok(&["open", &path]);
    }

    /// Open a fixture and wait for it to stabilize.
    pub fn open_fixture_wait(&self, fixture_name: &str, wait_text: &str) {
        let path = Self::fixture_path(fixture_name);
        self.run_ok(&["open", &path]);
        self.run_ok(&["wait", "--text", wait_text, "--timeout", "5000"]);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let bin = Self::bin_path();
        let _ = Command::new(&bin)
            .args(["close", "--session", &self.name])
            .output();
    }
}
