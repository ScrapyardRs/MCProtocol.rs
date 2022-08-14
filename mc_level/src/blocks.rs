use mc_serializer::primitive::VarInt;

pub struct Block {
    block_id: VarInt,
}

impl Block {

}

impl Into<VarInt> for Block {
    fn into(self) -> VarInt {
        self.block_id
    }
}

macro_rules! blocks {
    () => {
        todo!()
    }
}