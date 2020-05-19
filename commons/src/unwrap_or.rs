#[macro_export]
macro_rules! unwrap_or {
    ($option:expr, $expression:expr) => {
        match $option {
            Some(value) => value,
            None => $expression,
        }
    };
}

#[macro_export]
macro_rules! ok_or {
    ($option:expr, $expression:expr) => {
        match $option {
            Ok(value) => value,
            Err(..) => $expression,
        }
    };
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_unwrap_or_return() {
        fn function(mut_value: &mut usize, option: Option<usize>) {
            let value = unwrap_or!(option, return);
            *mut_value = value;
        }

        let mut mut_value = 0;
        function(&mut mut_value, None);
        assert_eq!(mut_value, 0);
        function(&mut mut_value, Some(3));
        assert_eq!(mut_value, 3);
    }

    #[test]
    fn test_unwrap_or_return_value() {
        fn function(option: Option<usize>) -> usize {
            let value = unwrap_or!(option, return 0);
            value + 1
        }

        assert_eq!(function(None), 0);
        assert_eq!(function(Some(2)), 3);
    }

    #[test]
    fn test_unwrap_or_continue() {
        fn function(mut_value: &mut usize, option: Option<usize>) {
            for _ in 0..1 {
                let value = unwrap_or!(option, continue);
                *mut_value = value;
            }
            *mut_value += 1;
        }

        let mut mut_value = 0;
        function(&mut mut_value, None);
        assert_eq!(mut_value, 1);
        function(&mut mut_value, Some(3));
        assert_eq!(mut_value, 4);
    }

    #[test]
    fn test_ok_or_return() {
        fn function(mut_value: &mut usize, option: Result<usize, usize>) {
            let value = ok_or!(option, return);
            *mut_value = value;
        }

        let mut mut_value = 0;
        function(&mut mut_value, Err(0));
        assert_eq!(mut_value, 0);
        function(&mut mut_value, Ok(3));
        assert_eq!(mut_value, 3);
    }

    #[test]
    fn test_ok_or_return_value() {
        fn function(option: Result<usize, usize>) -> usize {
            let value = ok_or!(option, return 0);
            value + 1
        }

        assert_eq!(function(Err(0)), 0);
        assert_eq!(function(Ok(2)), 3);
    }

    #[test]
    fn test_ok_or_continue() {
        fn function(mut_value: &mut usize, option: Result<usize, usize>) {
            for _ in 0..1 {
                let value = ok_or!(option, continue);
                *mut_value = value;
            }
            *mut_value += 1;
        }

        let mut mut_value = 0;
        function(&mut mut_value, Err(0));
        assert_eq!(mut_value, 1);
        function(&mut mut_value, Ok(3));
        assert_eq!(mut_value, 4);
    }
}
