use mc_serializer::primitive::VarInt;

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct ItemStack {
    pub item_id: VarInt,
    pub count: u8,
    #[nbt(inject_header)]
    pub item_tag: nbt::Blob,
}

fn main() {

}