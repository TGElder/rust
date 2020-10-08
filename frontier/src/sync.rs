macro_rules! sync {
    ($async_expr:expr) => {
        commons::futures::executor::block_on(async { $async_expr.await })
    };
}

#[cfg(test)]
mod tests {

    #[test]
    fn test() {
        async fn ret7() -> usize {
            7
        }
        assert_eq!(sync!(ret7()), 7);
    }
}
