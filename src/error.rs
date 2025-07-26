/// エラーケースを列挙
#[derive(Debug)]
pub enum MyError {}

/// Result<T, MyError> を簡潔に書くためのエイリアス
pub type MyResult<T> = Result<T, MyError>;
