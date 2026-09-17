//! Regressions for parser behavior that the interactive terminal depends on.
#[test]
fn deep_scrollback_is_bounded_and_keeps_the_users_position() {
    let mut parser = vt100::Parser::new(2, 10, 20);
    for line in 0..20 {
        parser.process(format!("{line:02}\r\n").as_bytes());
    }
    parser.screen_mut().set_scrollback(10);
    assert_eq!(parser.screen().contents(), "09\n10");
    parser.process(b"20\r\n");
    assert_eq!(parser.screen().contents(), "09\n10");
    for line in 21..60 {
        parser.process(format!("{line:02}\r\n").as_bytes());
    }
    assert!(parser.screen().scrollback() <= 20);
    assert_eq!(parser.screen().contents().lines().count(), 2);
}
