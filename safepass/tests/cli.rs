use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn setup_cmd() -> (Command, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::new(assert_cmd::cargo_bin!("safepass"));

    cmd.env("SAFEPASS_STORE_DIR", temp_dir.path())
        .env("SAFEPASS_TEST_MODE", "1");

    (cmd, temp_dir)
}

#[test]
fn test_add_command() {
    let (mut cmd, _temp_dir) = setup_cmd();

    // Simulate user input: service password twice
    let input = "service_pass\nservice_pass\n";

    cmd.arg("add")
        .arg("--service")
        .arg("google")
        .arg("--username")
        .arg("user@gmail.com")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Password for google added successfully.",
        ));

    // Verify it exists by trying to add again (should fail)
    let mut cmd2 = Command::new(assert_cmd::cargo_bin!("safepass"));
    cmd2.env("SAFEPASS_STORE_DIR", _temp_dir.path())
        .env("SAFEPASS_TEST_MODE", "1");

    cmd2.arg("add")
        .arg("--service")
        .arg("google")
        .arg("--username")
        .arg("user@gmail.com")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_get_command() {
    let (mut cmd, _temp_dir) = setup_cmd();

    // Add entry first
    let input = "secret123\nsecret123\n";
    cmd.arg("add")
        .arg("--service")
        .arg("github")
        .arg("--username")
        .arg("dev")
        .write_stdin(input)
        .assert()
        .success();

    // Get entry
    let mut cmd2 = Command::new(assert_cmd::cargo_bin!("safepass"));
    cmd2.env("SAFEPASS_STORE_DIR", _temp_dir.path())
        .env("SAFEPASS_TEST_MODE", "1");

    cmd2.arg("get")
        .arg("--service")
        .arg("github")
        .arg("--username")
        .arg("dev")
        .assert()
        .success()
        .stdout(predicate::str::contains("Password: secret123"));
}

#[test]
fn test_update_command() {
    let (mut cmd, _temp_dir) = setup_cmd();

    // Add entry
    let input = "old\nold\n";
    cmd.arg("add")
        .arg("--service")
        .arg("twitter")
        .arg("--username")
        .arg("bird")
        .write_stdin(input)
        .assert()
        .success();

    // Update entry
    let mut cmd2 = Command::new(assert_cmd::cargo_bin!("safepass"));
    cmd2.env("SAFEPASS_STORE_DIR", _temp_dir.path())
        .env("SAFEPASS_TEST_MODE", "1");

    let update_input = "new\nnew\n";
    cmd2.arg("update")
        .arg("--service")
        .arg("twitter")
        .arg("--username")
        .arg("bird")
        .write_stdin(update_input)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Password for twitter updated successfully.",
        ));

    // Verify update
    let mut cmd3 = Command::new(assert_cmd::cargo_bin!("safepass"));
    cmd3.env("SAFEPASS_STORE_DIR", _temp_dir.path())
        .env("SAFEPASS_TEST_MODE", "1");

    cmd3.arg("get")
        .arg("--service")
        .arg("twitter")
        .arg("--username")
        .arg("bird")
        .assert()
        .success()
        .stdout(predicate::str::contains("Password: new"));
}

#[test]
fn test_delete_command() {
    let (mut cmd, _temp_dir) = setup_cmd();

    // Add entry
    let input = "pass\npass\n";
    cmd.arg("add")
        .arg("--service")
        .arg("netflix")
        .arg("--username")
        .arg("chill")
        .write_stdin(input)
        .assert()
        .success();

    // Delete entry
    let mut cmd2 = Command::new(assert_cmd::cargo_bin!("safepass"));
    cmd2.env("SAFEPASS_STORE_DIR", _temp_dir.path())
        .env("SAFEPASS_TEST_MODE", "1");

    cmd2.arg("delete")
        .arg("--service")
        .arg("netflix")
        .arg("--username")
        .arg("chill")
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted successfully"));

    // Verify deletion
    let mut cmd3 = Command::new(assert_cmd::cargo_bin!("safepass"));
    cmd3.env("SAFEPASS_STORE_DIR", _temp_dir.path())
        .env("SAFEPASS_TEST_MODE", "1");

    cmd3.arg("get")
        .arg("--service")
        .arg("netflix")
        .arg("--username")
        .arg("chill")
        .assert()
        .failure() // Should fail or exit 1
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_list_command() {
    let (mut cmd, _temp_dir) = setup_cmd();

    // List empty
    cmd.arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No entries found"));

    // Add entry
    let mut cmd2 = Command::new(assert_cmd::cargo_bin!("safepass"));
    cmd2.env("SAFEPASS_STORE_DIR", _temp_dir.path())
        .env("SAFEPASS_TEST_MODE", "1");

    let input = "pass\npass\n";
    cmd2.arg("add")
        .arg("--service")
        .arg("amazon")
        .arg("--username")
        .arg("prime")
        .write_stdin(input)
        .assert()
        .success();

    // List again
    let mut cmd3 = Command::new(assert_cmd::cargo_bin!("safepass"));
    cmd3.env("SAFEPASS_STORE_DIR", _temp_dir.path())
        .env("SAFEPASS_TEST_MODE", "1");

    cmd3.arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("amazon"))
        .stdout(predicate::str::contains("prime"));
}
