use crate::SerialError;
use core::future::Future;
pub trait Write: SerialError {
    /// Future returned by the `write` method.
    type WriteAllFuture<'a>: Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;
    
    /// Future returned by the `flush` method.
    type FlushFuture<'a>: Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;

    /// Writes a single word to the serial interface
    fn write_all<'a>(&'a mut self, words: &'a [u8]) -> Self::WriteAllFuture<'a>;

    /// Ensures that none of the previously written words are still buffered
    fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a>;
}