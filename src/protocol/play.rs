pub mod sb {}

pub mod cb {
    use crate::chat::Chat;

    #[derive(drax_derive::DraxTransport)]
    pub struct Disconnect {
        #[drax(json = 32767)]
        pub reason: Chat,
    }

    crate::import_registrations! {
        Disconnect {
            760 -> 0x19,
        }
    }
}
