use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use std::convert::TryFrom;
use syn::{parse_macro_input, ItemEnum};

const DEFAULT_SUPER_ADMIN_NAME: &str = "__SUPER_ADMIN";
const DEFAULT_BITFLAGS_TYPE_NAME: &str = "RoleFlags";

pub fn derive_access_control_role(input: TokenStream) -> TokenStream {
    // This derive doesn't take attributes, so no need to use `darling`.
    let input: ItemEnum = parse_macro_input!(input);
    let ItemEnum {
        ident, variants, ..
    } = input;

    // TODO cleanup by using range (see bitflags_idxs below)
    let (variant_idxs, variant_items): (Vec<_>, Vec<_>) =
        variants.iter().cloned().enumerate().unzip();
    let variant_idxs = variant_idxs
        .iter()
        .map(|&idx| {
            u8::try_from(idx).expect("The number of variants should be representable by u8")
        })
        .collect::<Vec<_>>();
    let variant_names = variants
        .iter()
        .map(|v| format!("{}", v.ident))
        .collect::<Vec<_>>();

    let bitflags_type_ident = Ident::new(DEFAULT_BITFLAGS_TYPE_NAME, Span::call_site());
    let bitflags_idents = bitflags_idents(variant_names.as_ref(), bitflags_type_ident.span());
    let bitflags_idxs = 0..u8::try_from(bitflags_idents.len())
        .expect("The number of bitflags should be representable by u8");

    let output = quote! {
        impl From<#ident> for u8 {
            fn from(value: #ident) -> Self {
                match value {
                    #(
                        #ident::#variant_items => #variant_idxs,
                    )*
                }
            }
        }

        impl ::std::convert::TryFrom<u8> for #ident {
            type Error = &'static str;

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    #(
                        #variant_idxs => Ok(#ident::#variant_items),
                    )*
                    _ => Err("Value does not correspond to a variant"),
                }
            }
        }

        impl From<#ident> for &'static str {
            fn from(value: #ident) -> Self {
                match value {
                    #(
                        #ident::#variant_items => #variant_names,
                    )*
                }
            }
        }

        impl ::std::convert::TryFrom<&str> for #ident {
            type Error = &'static str;

            fn try_from(value: &str) -> Result<#ident, Self::Error> {
                match value {
                    #(
                        #variant_names => Ok(#ident::#variant_items),
                    )*
                    _ => Err("Value does not correspond to a variant"),
                }
            }
        }

        /// Panics if `n` is too large.
        fn safe_leftshift(value: u128, n: u8) -> u128 {
            value
                .checked_shl(n.into())
                .expect("Too many enum variants to be represented by bitflags")
        }

        // TODO explain enum<->bitflag conversion
        impl AccessControlRole for #ident {
            fn acl_super_admin_permission() -> u128 {
                safe_leftshift(1, 0)
            }

            fn acl_permission(self) -> u128 {
                let n = (u8::from(self) + 1)
                    .checked_mul(2)
                    .expect("Too many enum variants") - 1;
                safe_leftshift(1, n)
            }

            fn acl_admin_permission(self) -> u128 {
                let n = (u8::from(self) + 1)
                    .checked_mul(2)
                    .expect("Too many enum variants");
                safe_leftshift(1, n)
            }
        }

        ::bitflags::bitflags! {
            #[derive(BorshDeserialize, BorshSerialize, Default)]
            struct #bitflags_type_ident: u128 {
                #(
                    const #bitflags_idents = 1u128 << #bitflags_idxs;
                )*
            }
        }
    };

    output.into()
}

fn bitflags_idents(names: &[String], span: Span) -> Vec<Ident> {
    // Assuming enum variant names are in camel case, simply converting them
    // to uppercase is not ideal. However, bitflag identifiers aren't exposed,
    // so let's not bother with converting camel to screaming-snake case.
    let names = names
        .iter()
        .map(|name| name.to_uppercase())
        .collect::<Vec<_>>();
    let admin_names = names
        .iter()
        .map(|name| format!("{}_ADMIN", name))
        .collect::<Vec<_>>();
    let mut idents = vec![Ident::new(DEFAULT_SUPER_ADMIN_NAME, span.clone())];
    for (name, admin_name) in names.iter().zip(admin_names) {
        idents.push(Ident::new(name.as_ref(), span.clone()));
        idents.push(Ident::new(admin_name.as_ref(), span.clone()));
    }
    idents
}
