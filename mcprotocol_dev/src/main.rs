use mc_serializer::primitive::VarInt;
use mc_serializer::serde::Contextual;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Ok(())
}

mc_serializer::auto_string!(SomeString, 255);

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
struct SomeJsonStruct {
    test: String,
}

impl Contextual for SomeJsonStruct {
    fn context() -> String {
        format!("SomeJsonStruct")
    }
}

// #[derive(mc_serializer_derive::Serial)]
// struct TestStruct {
//     some_field: VarInt,
//     #[serial_if(protocol_version >= ProtocolVersion::V119R1)]
//     #[default(SomeString::from("not version 1.19 lol sad"))]
//     only_version_119_above: SomeString,
//     #[json(256)] // 256 max characters to build the struct - anything over should be ignored
//     #[serial_if(protocol_version >= ProtocolVersion::V119R1)]
//     #[default(SomeJsonStruct { test: format ! ("Default") })]
//     some_json_thing: SomeJsonStruct,
//     #[nbt] // this struct will be serialized / deserialized as an NBT object, such as item meta :)
//     #[serial_if(protocol_version >= ProtocolVersion::V119R1)]
//     #[default(SomeJsonStruct { test: format ! ("Default") })]
//     some_nbt_thing: SomeJsonStruct,
// }

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
enum ExampleEnum {
    #[key(VarInt::from(0))]
    Option1,
    #[key(VarInt::from(1))]
    Option2(VarInt, SomeString, #[json(256)] SomeJsonStruct),
    #[key(VarInt::from(2))]
    Option3 {
        item1: VarInt,
        item2: SomeString,
        #[nbt]
        item3: SomeJsonStruct,
    },
}

