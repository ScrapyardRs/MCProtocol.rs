use drax::nbt::CompoundTag;
use drax::prelude::{AsyncRead, AsyncWrite, PacketComponent, Size};
use drax::transport::packet::primitive::VarInt;
use std::future::Future;
use std::mem::size_of;
use std::pin::Pin;

const MULTIPLY_DE_BRUIJN_BIT_POSITION: [i32; 32] = [
    0, 1, 28, 2, 29, 14, 24, 3, 30, 22, 20, 15, 25, 17, 4, 8, 31, 27, 13, 23, 21, 19, 16, 7, 26,
    12, 18, 6, 11, 5, 10, 9,
];

pub const fn ceil_log_2(n: i32) -> i32 {
    let n = if is_power_of_2(n) { n } else { shift_2(n) };
    MULTIPLY_DE_BRUIJN_BIT_POSITION
        [((((n as u64 * 125613361u64) >> 27u64) as i32) & 0x1Fi32) as usize]
}

const fn is_power_of_2(n: i32) -> bool {
    n != 0i32 && (n & (n - 1i32)) == 0i32
}

const fn shift_2(n: i32) -> i32 {
    let mut n2 = n - 1i32;
    n2 |= n2 >> 1i32;
    n2 |= n2 >> 2i32;
    n2 |= n2 >> 4i32;
    n2 |= n2 >> 8i32;
    n2 |= n2 >> 16i32;
    n2 + 1i32
}

const fn log2(n: i32) -> i32 {
    ceil_log_2(n) - if is_power_of_2(n) { 0i32 } else { 1i32 }
}

#[derive(Debug)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    const PACKED_Z_LENGTH: i32 = (1i32 + log2(shift_2(30000000)));
    const PACKED_X_LENGTH: i32 = (Self::PACKED_Z_LENGTH) as i32;
    const PACKED_Y_LENGTH: i32 =
        (64u64 - Self::PACKED_X_LENGTH as u64 - Self::PACKED_Z_LENGTH as u64) as i32;
    const PACKED_X_MASK: i64 = ((1u64 << Self::PACKED_X_LENGTH) - 1u64) as i64;
    const PACKED_Y_MASK: i64 = ((1u64 << Self::PACKED_Y_LENGTH) - 1u64) as i64;
    const PACKED_Z_MASK: i64 = ((1u64 << Self::PACKED_Z_LENGTH) - 1u64) as i64;
    const Z_OFFSET: i32 = (Self::PACKED_Y_LENGTH) as i32;
    const X_OFFSET: i32 = (Self::PACKED_Y_LENGTH + Self::PACKED_Z_LENGTH) as i32;
}

impl<C> PacketComponent<C> for BlockPos {
    type ComponentType = BlockPos;

    fn decode<'a, A: AsyncRead + Unpin + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> Pin<Box<dyn Future<Output = drax::prelude::Result<Self::ComponentType>> + 'a>> {
        Box::pin(async move {
            let value = i64::decode(context, read).await?;
            let x = (value << (64 - Self::X_OFFSET - Self::PACKED_X_LENGTH)
                >> (64 - Self::PACKED_X_LENGTH)) as i32;
            let y = (value << (64 - Self::PACKED_Y_LENGTH) >> (64 - Self::PACKED_Y_LENGTH)) as i32;
            let z = (value << (64 - Self::Z_OFFSET - Self::PACKED_Z_LENGTH)
                >> (64 - Self::PACKED_Z_LENGTH)) as i32;
            Ok(Self { x, y, z })
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> Pin<Box<dyn Future<Output = drax::prelude::Result<()>> + 'a>> {
        Box::pin(async move {
            let mut value: i64 = 0;
            value |= (component_ref.x as i64 & Self::PACKED_X_MASK) << Self::X_OFFSET;
            value |= component_ref.y as i64 & Self::PACKED_Y_MASK;
            value |= (component_ref.z as i64 & Self::PACKED_Z_MASK) << Self::Z_OFFSET;
            i64::encode(&value, context, write).await
        })
    }

    fn size(_: &Self::ComponentType, _: &mut C) -> drax::prelude::Result<Size> {
        Ok(Size::Constant(size_of::<u64>()))
    }
}

registry! {
    components {
        struct SimpleLocation {
            x: f64,
            y: f64,
            z: f64
        },

        struct Location {
            inner_loc: SimpleLocation,
            yaw: f64,
            pitch: f64
        },

        struct ItemStack {
            item_id: VarInt,
            item_data: u8,
            tag: Option<CompoundTag>
        },

        struct GlobalPos {
            dimension: String,
            pos: BlockPos
        },

        enum InteractionHand<key: VarInt> {
            MainHand {},
            OffHand {}
        }
    }
}
