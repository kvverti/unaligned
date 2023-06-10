//! Types encapsulating values stored in unaligned storage.

pub mod unaligned;
pub mod cell;

pub use unaligned::Unaligned;

#[cfg(test)]
mod tests {
    use super::*;
}
