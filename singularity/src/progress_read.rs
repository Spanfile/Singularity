use crossbeam_utils::atomic::AtomicCell;
use std::io::Read;

pub struct ProgressRead<'a, R>
where
    R: Read,
{
    reader: R,
    counter: &'a AtomicCell<u64>,
}

impl<'a, R> ProgressRead<'a, R>
where
    R: Read,
{
    pub fn new(reader: R, counter: &'a AtomicCell<u64>) -> Self {
        Self { reader, counter }
    }
}

impl<'a, R> Read for ProgressRead<'a, R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let amt = self.reader.read(buf)?;
        self.counter.fetch_add(amt as u64);
        Ok(amt)
    }
}
