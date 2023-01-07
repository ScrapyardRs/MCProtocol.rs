use drax::transport::packet::primitive::VarInt;
use drax::transport::packet::string::LimitedString;

registry! {
    components {
        /// Defines an intention; where the user is routed depends on the value of this.
        enum ConnectionProtocol<key: VarInt> {
            /// Denotes the "play" phase, see [`crate::clientbound::play`] and
            /// [`crate::serverbound::play`]  for more information.
            Play {},
            /// Denotes the "status" phase, see [`crate::clientbound::status`] and
            /// [`crate::serverbound::status`]  for more information. <br />
            ///
            /// In the `status` phase the client will request the server's status and ping the
            /// server to gauge latency.
            Status {},
            /// Denotes the "login" phase, see [`crate::clientbound::login`] and
            /// [`crate::serverbound::login`]  for more information. <br />
            ///
            /// In the `login` phase the client will be authenticated; this phase ends once the
            /// server sends a `LoginSuccess` packet.
            Login {}
        }
    }

    registry HandshakingRegistry {
        /// Base packet for initiating a connection.
        struct ClientIntention {
            /// The client version of the inbound connection
            protocol_version: VarInt,
            /// The address the client used to connect to the server
            host_name: LimitedString<255>,
            /// The port the client used to connect to the server
            port: u16,
            /// The next state the client intends to be in
            intention: ConnectionProtocol
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::handshaking::{ConnectionProtocol, HandshakingRegistry};
    use drax::prelude::DraxWriteExt;
    use std::io::Cursor;

    #[tokio::test]
    pub async fn regression_test() -> drax::prelude::Result<()> {
        let mut cursor = Cursor::new(vec![]);
        assert!(matches!(
            cursor
                .encode_component::<(), HandshakingRegistry>(
                    &mut (),
                    &HandshakingRegistry::ClientIntention {
                        protocol_version: 754,
                        host_name: "localhost".to_string(),
                        port: 25565,
                        intention: ConnectionProtocol::Play {},
                    },
                )
                .await,
            Ok(_)
        ));
        Ok(())
    }
}
