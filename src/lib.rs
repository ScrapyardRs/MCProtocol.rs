#![feature(macro_metavar_expr)]

macro_rules! registry_internal {
    ($(#[$($tt:tt)*])* enum $enum_name:ident {
        $key_name:ident: $key_delegate_type:ty,
        $(@ser_delegate $static_product_delegate_type:ty,)?
        $(@match $key_matcher:expr,)?
        $(
            $(#[$($vtt:tt)*])*
            $($key_matcher_case:literal =>)? $variant_name:ident {
                $(
                    $(
                        $(#[$($ftt:tt)*])*
                        $field_name:ident: $delegate_type:ty,
                    )+
                )?
            }
        ),*
    }) => {
        drax::enum_packet_components! {
            $(#[$($tt)*])*
            $enum_name {
                $key_name: $key_delegate_type,
                $(@ser_delegate $static_product_delegate_type,)?
                $(@match $key_matcher,)?
                $(
                    $(#[$($vtt)*])*
                    $($key_matcher_case =>)? $variant_name {
                        $(
                            $(
                                $(#[$($ftt)*])*
                                $field_name: $delegate_type
                            ),+
                        )?
                    }
                ),*
            }
        }
    };
    ($(#[$($tt:tt)*])* struct $struct_name:ident {
        $(
            $(
                $(#[$($doc_tt:tt)*])*
                $field_name:ident: $delegate_type:ty,
            )+
        )?
    }) => {
        drax::struct_packet_components! {
            $(#[$($tt)*])*
            $struct_name {
                $(
                    $(
                        $(#[$($doc_tt)*])*
                        $field_name: $delegate_type
                    ),+
                )?
            }
        }
    };
}

macro_rules! registry {
    (
        $(
        components {
            $(
                $(#[$($tt2:tt)*])* // any extra attributes for the struct
                $(enum $component_enum_name:ident<$(C: $c_e_ctx_ty:ty,)? $c_key_name:ident: $c_key_delegate_type:ty> {
                    $(@ser_delegate $c_static_product_delegate_type:ty,)?
                    $(@match $c_key_matcher:expr,)?
                    $( // enum field delegations
                        $(#[$($cvtt:tt)*])*
                        $c_variant_name:ident {
                            $(@key($c_key_matcher_case:literal);)?
                            $(
                                $(
                                    $(#[$($cftt:tt)*])*
                                    $c_v_field_name:ident: $c_v_delegate_type:ty
                                ),+
                            )?
                        }
                    ),*
                })?
                $(struct $component_struct_name:ident $(<$c_ctx_ty:ty>)? {
                    $( // struct field delegations
                        $(
                            $(#[$($c_doc_tt:tt)*])*
                            $c_field_name:ident: $c_delegate_type:ty
                        ),+
                    )?
                })?
            ),*
        }
        )?
        $(
        $(#[$($registry_attrs:tt)*])*
        registry $registry_name:ident {
            $(
                $(#[$($tt:tt)*])* // any extra attributes for the struct
                $(enum $enum_name:ident<$(C: $e_ctx_ty:ty,)? $key_name:ident: $key_delegate_type:ty> {
                    $(@ser_delegate $static_product_delegate_type:ty,)?
                    $(@match $key_matcher:expr,)?
                    $( // enum field delegations
                        $(#[$($vtt:tt)*])*
                        $variant_name:ident {
                            $(@key($key_matcher_case:literal))?
                            $(
                                $(
                                    $(#[$($ftt:tt)*])*
                                    $v_field_name:ident: $v_delegate_type:ty
                                ),+
                            )?
                        }
                    ),*
                })?
                $(struct $struct_name:ident $(<$ctx_ty:ty>)? {
                    $( // struct field delegations
                        $(
                            $(#[$($doc_tt:tt)*])*
                            $field_name:ident: $delegate_type:ty
                        ),+
                    )?
                })?
            ),*
        })?
    ) => {
        $($(registry_internal! {
            $(#[$($tt2)*])*
            $(enum $component_enum_name$(<$c_e_ctx_ty>)? {
                $c_key_name: $c_key_delegate_type,
                $(@ser_delegate $c_static_product_delegate_type,)?
                $(@match $c_key_matcher,)?
                $( // enum field delegations
                    $(#[$($cvtt)*])*
                    $($c_key_matcher_case =>)? $c_variant_name {
                        $(
                            $(
                                $(#[$($cftt)*])*
                                $c_v_field_name: $c_v_delegate_type,
                            )+
                        )?
                    }
                ),*
            })?
            $(struct $component_struct_name$(<$c_ctx_ty>)? {
            $( // struct field delegations
                $(
                    $(#[$($c_doc_tt)*])*
                    $c_field_name: $c_delegate_type,
                )+
            )?
            })?
        })*)?
        $($(registry_internal! {
            $(#[$($tt)*])*
            $(enum $enum_name$(<$e_ctx_ty>)? {
                $key_name: $key_delegate_type,
                $(@ser_delegate $static_product_delegate_type,)?
                $(@match $key_matcher,)?
                $( // enum field delegations
                    $(#[$($vtt)*])*
                    $($key_matcher_case =>)? $variant_name {
                    $($(
                        $(#[$($ftt)*])*
                        $v_field_name: $v_delegate_type,
                    )+)?
                    },
                )*
            })?
            $(struct $struct_name$(<$ctx_ty>)? {$($( // struct field delegations
                $(#[$($doc_tt)*])*
                $field_name: $delegate_type,
            )+)?})?
        })*)?
        $(
        drax::enum_packet_components! {
            $(#[$($registry_attrs)*])*
            ///
            /// Auto generated registry; <br />
            ///
            /// The following are all packets of the provided registry; indexed by their packet
            /// ID.
            $registry_name {
                key: drax::transport::packet::primitive::VarInt,
                $(
                    $(
                    /// Wrapper variant for struct
                    #[doc = concat!(
                        "<code style=\"white-space: nowrap\"><a href=\"./struct.",
                        stringify!($struct_name),
                        ".html\">",
                        stringify!($struct_name),
                        "</a></code>",
                    )]
                    $struct_name {
                        /// Inner wrapper type for below fields. See type def for more info.
                        $(
                        // purposefully break html syntax
                        /// </td> </tr> </tbody> </table>
                        ///
                        /// Packet
                        #[doc=stringify!($struct_name)]
                        /// layout:
                        ///
                        /// <table style="display=flex; justify-content: start; width: 100%">
                        /// <thead>
                        ///     <tr>
                        ///         <th>Field</th>
                        ///         <th>Description</th>
                        ///     </tr>
                        /// </thead>
                        /// <tbody>
                        $(
                            #[doc=concat!(
                                "<tr><td>",
                                stringify!($field_name),
                                "</td><td>"
                            )]
                            #[doc=drax::expand_field!(@internal @doc $(#[$($doc_tt)*])*)]
                            $(#[$($doc_tt)*])*
                        )+
                        )?
                        inner: Box<$struct_name>
                    })?
                    $(
                    /// Wrapper variant for enum
                    #[doc = concat!(
                        "<code style=\"white-space: nowrap\"><a href=\"./enum.",
                        stringify!($enum_name),
                        ".html\">",
                        stringify!($enum_name),
                        "</a></code>",
                    )]
                    $enum_name {
                        /// Inner linkage
                        inner: Box<$enum_name>
                    })?
                ),*
            }
        })?
    };
}

pub mod clientbound;
pub mod common;
pub mod handshaking;
pub mod serverbound;
