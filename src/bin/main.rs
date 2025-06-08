use my_shell::shell::*;

fn main() {
    // init
    // let origin_term = get_term();
    // set_raw_term(origin_term.clone());

    my_shell::init();
    let mut s = MyShell::new();
    s.command_mode();

    // end process
    // set_term(&origin_term);
}
