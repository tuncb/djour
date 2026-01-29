use assert_cmd::Command;

pub fn djour_cmd() -> Command {
    let mut cmd = Command::cargo_bin("djour").unwrap();
    cmd.env_remove("DJOUR_ROOT");
    cmd.env_remove("DJOUR_MODE");
    cmd.env_remove("EDITOR");
    cmd.env_remove("VISUAL");
    cmd
}
