#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixDomain {
    /// Fix is in the caller's input or usage.
    Client,
    /// Fix requires repairing stored content (corrupt or malformed).
    Data,
    /// Fix is in the underlying infrastructure (permissions, disk, network).
    Infra,
    /// Fix is in Pari's code (invariant violated, logic bug).
    Pari,
}
