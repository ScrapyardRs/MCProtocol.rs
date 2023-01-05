registry! {
    registry ServerboundStatusRegistry {
        /// The request for a [crate::clientbound::status::Response]
        struct Request {
        },

        /// Pings the server requesting a [crate::clientbound::status::Pong]
        /// back with the same payload.
        struct Ping {
            /// The payload of the ping
            payload: u64
        }
    }
}
