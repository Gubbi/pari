pub enum AssetOp<E> {
    Put(E),
    Post(E),
    Patch(E),
    Delete,
    Get,
    Head,
}

pub struct AssetRequest<L, E> {
    pub location: L,
    pub op: AssetOp<E>,
}

pub enum AssetResponse<E> {
    Done,
    Data(E),
    Exists(bool),
}
