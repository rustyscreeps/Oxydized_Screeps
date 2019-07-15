pub mod kernel;
pub mod process;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_a_process() {
        assert_eq!(2 + 2, 4);
    }
}
