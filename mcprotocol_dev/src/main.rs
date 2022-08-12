use mc_serializer::contextual;
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::Deserialize;
use mc_serializer::serde::ProtocolVersion;
use std::io::Cursor;

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct ItemStack {
    pub item_id: VarInt,
    pub count: u8,
    #[nbt]
    pub item_tag: nbt::Blob,
}

fn main() {
    let mut blob = nbt::Blob::new();
    blob.insert("example1", "test").expect("");

    let item_stack = ItemStack {
        item_id: VarInt::from(10),
        count: 12,
        item_tag: blob,
    };

    println!("{:?}", item_stack);

    let mut buffer = Vec::new();
    mc_serializer::serde::Serialize::serialize(&item_stack, &mut buffer, ProtocolVersion::V119_1)
        .unwrap();

    let mut buffer = Cursor::new(buffer);
    let out = ItemStack::deserialize(&mut buffer, ProtocolVersion::V119_1).unwrap();

    println!("{:?}", out)
}
