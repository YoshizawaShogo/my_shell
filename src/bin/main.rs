fn main() {
    my_shell::init();
    let mut s = my_shell::shell::MyShell::new();
    s.command_mode().unwrap();
}
