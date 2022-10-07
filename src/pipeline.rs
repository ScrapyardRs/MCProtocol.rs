// use drax::link;
// use drax::transport::frame::FrameDecoder;
// use drax::transport::frame::PacketFrame;
// use drax::transport::pipeline::{ChainProcessor, ProcessChainLink};
// use drax::transport::TransportProcessorContext;
//
// pub struct MinecraftProtocolPipeline<PacketOutput> {
//     processor_context: TransportProcessorContext,
//     chain_processor: ProcessChainLink<Vec<u8>, PacketFrame, PacketOutput>,
// }
//
// impl<PacketOutput: Clone> MinecraftProtocolPipeline<PacketOutput> {
//     pub fn create<
//         Reg: PacketRegistry<PacketOutput> + ChainProcessor<Input = PacketFrame, Output = PacketOutput>,
//     >(
//         reg: Reg,
//         compression_threshold: isize,
//     ) -> Self {
//         let chain_processor = FrameDecoder::new(compression_threshold);
//         // let chain_processor = link!(chain_processor, reg);
//         // Self {
//         //     processor_context: TransportProcessorContext::new(),
//         //     chain_processor,
//         // }
//         todo!()
//     }
// }
