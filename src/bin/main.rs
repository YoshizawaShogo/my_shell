use my_shell::shell::*;

fn main() {
    my_shell::init();
    let mut s = MyShell::new();
    s.command_mode();
}
