pub mod abbr;
pub mod alias;
pub mod builtin;
pub mod execute;
pub mod parse;
pub mod tokenize;
pub mod util;

#[cfg(test)]
mod tests {
    use crate::command::tokenize::Token;
    use crate::command::tokenize::tokenize;

    #[test]
    fn test_simple_words() {
        assert_eq!(
            tokenize("echo hello"),
            vec![Token::Word("echo".into()), Token::Word("hello".into()),]
        );
    }

    #[test]
    fn test_quoted_words() {
        assert_eq!(
            tokenize("\"\\\"a'a\\\"\""),
            vec![Token::Word("\\\"a'a\\\"".into())]
        );
        assert_eq!(tokenize("\"a'a\""), vec![Token::Word("a'a".into())]);
    }

    #[test]
    fn test_and_or() {
        assert_eq!(
            tokenize("a && b || c"),
            vec![
                Token::Word("a".into()),
                Token::And,
                Token::Word("b".into()),
                Token::Or,
                Token::Word("c".into()),
            ]
        );
    }

    #[test]
    fn test_redirects() {
        assert_eq!(
            tokenize("a > b"),
            vec![
                Token::Word("a".into()),
                Token::RedirectOut,
                Token::Word("b".into())
            ]
        );
        assert_eq!(
            tokenize("a >> b"),
            vec![
                Token::Word("a".into()),
                Token::RedirectAppend,
                Token::Word("b".into())
            ]
        );
        assert_eq!(
            tokenize("a &> b"),
            vec![
                Token::Word("a".into()),
                Token::RedirectBoth,
                Token::Word("b".into())
            ]
        );
        assert_eq!(
            tokenize("a &>> b"),
            vec![
                Token::Word("a".into()),
                Token::RedirectBothAppend,
                Token::Word("b".into())
            ]
        );
        assert_eq!(
            tokenize("a 2> b"),
            vec![
                Token::Word("a".into()),
                Token::RedirectErr,
                Token::Word("b".into())
            ]
        );
        assert_eq!(
            tokenize("a 2>> b"),
            vec![
                Token::Word("a".into()),
                Token::RedirectErrAppend,
                Token::Word("b".into())
            ]
        );
    }

    #[test]
    fn test_pipes() {
        assert_eq!(
            tokenize("a | b"),
            vec![
                Token::Word("a".into()),
                Token::Pipe,
                Token::Word("b".into())
            ]
        );
        assert_eq!(
            tokenize("a || b"),
            vec![Token::Word("a".into()), Token::Or, Token::Word("b".into())]
        );
        assert_eq!(
            tokenize("a &| b"),
            vec![
                Token::Word("a".into()),
                Token::PipeBoth,
                Token::Word("b".into())
            ]
        );
        assert_eq!(
            tokenize("a 2| b"),
            vec![
                Token::Word("a".into()),
                Token::PipeErr,
                Token::Word("b".into())
            ]
        );
    }

    #[test]
    fn test_mixed_tokens() {
        let input = "echo 'hello world' && ls -l | grep txt &> result.log";
        let expected = vec![
            Token::Word("echo".into()),
            Token::Word("hello world".into()),
            Token::And,
            Token::Word("ls".into()),
            Token::Word("-l".into()),
            Token::Pipe,
            Token::Word("grep".into()),
            Token::Word("txt".into()),
            Token::RedirectBoth,
            Token::Word("result.log".into()),
        ];
        assert_eq!(tokenize(input), expected);
    }

    #[test]
    fn test_no_space_operators() {
        let input = "a && b || c | d &> e &>> f 2> g 2>> h 2| i";
        let expected = vec![
            Token::Word("a".into()),
            Token::And,
            Token::Word("b".into()),
            Token::Or,
            Token::Word("c".into()),
            Token::Pipe,
            Token::Word("d".into()),
            Token::RedirectBoth,
            Token::Word("e".into()),
            Token::RedirectBothAppend,
            Token::Word("f".into()),
            Token::RedirectErr,
            Token::Word("g".into()),
            Token::RedirectErrAppend,
            Token::Word("h".into()),
            Token::PipeErr,
            Token::Word("i".into()),
        ];
        assert_eq!(tokenize(input), expected);
    }
}
