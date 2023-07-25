#[cfg(test)]
mod parameter_test {
    use std::fs::{remove_file, write};
    use crate::parse_args;

    #[test]
    fn test_validate_args_success() {
        /*
        let test_file = "/tmp/test_urls.txt";
        if write(&test_file, "a\nb\nc").is_ok() {
            let args = vec!["program".to_string(), "-u".to_string(), "http://test.xxx/FUZZ".to_string(),
                            "-w".to_string(), test_file.to_owned()];
            dbg!(&parse_args(&args));
            assert!(parse_args(&args).is_ok());
        }
        let _ = remove_file(test_file);
        */
    }

    #[test]
    fn test_validate_args_failure() {
        let args = vec!["program".to_string(), "-u".to_string(), "http://test.xxx/xxx".to_string()];
        assert!(parse_args(&args).is_err());
        let args = vec!["program".to_string(), "-u".to_string(), "http://test.xxx/FUZZ".to_string()];
        assert!(parse_args(&args).is_err());
        let args = vec!["program".to_string(), "-u".to_string(), "http://test.xxx/FUZZ".to_string(),
                        "-w".to_string(), "xxxx".to_string()];
        //dbg!(&parse_args(&args));
        assert!(parse_args(&args).is_err());
    }
}