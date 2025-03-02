use crate::core_impl::MethodKind;
use crate::{BindgenArgType, ImplItemMethodInfo, SerializerType};

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::ReturnType;

impl ImplItemMethodInfo {
    /// Generates metadata struct for this method.
    ///
    /// # Example:
    /// The following method:
    /// ```ignore
    /// fn f3(&mut self, arg0: FancyStruct, arg1: u64) -> Result<IsOk, Error> { }
    /// ```
    /// will produce this struct:
    /// ```ignore
    /// near_sdk::__private::MethodMetadata {
    ///     name: "f3".to_string(),
    ///     is_view: false,
    ///     is_init: false,
    ///     args: {
    ///         #[derive(borsh::BorshSchema)]
    ///         #[derive(serde :: Deserialize, serde :: Serialize)]
    ///         struct Input {
    ///             arg0: FancyStruct,
    ///             arg1: u64,
    ///         }
    ///         Some(Input::schema_container())
    ///     },
    ///     callbacks: vec![],
    ///     callbacks_vec: None,
    ///     result: Some(Result < IsOk, Error > ::schema_container())
    /// }
    /// ```
    /// If args are serialized with Borsh it will not include `#[derive(borsh::BorshSchema)]`.
    pub(crate) fn metadata_struct(&self) -> TokenStream2 {
        let method_name_str = self.attr_signature_info.ident.to_string();
        let is_view = matches!(&self.attr_signature_info.method_kind, &MethodKind::View(_));
        let is_init = matches!(&self.attr_signature_info.method_kind, &MethodKind::Init(_));
        let args = if self.attr_signature_info.input_args().next().is_some() {
            let input_struct = self.attr_signature_info.input_struct_deser();
            // If input args are JSON then we need to additionally specify schema for them.
            let additional_schema = match &self.attr_signature_info.input_serializer {
                SerializerType::Borsh => TokenStream2::new(),
                SerializerType::JSON => quote! {
                    #[derive(::borsh::BorshSchema)]
                },
            };
            quote! {
                {
                    #additional_schema
                    #[allow(dead_code)]
                    #input_struct
                    ::std::option::Option::Some(<Input as ::near_sdk::borsh::BorshSchema>::schema_container())
                }
            }
        } else {
            quote! {
                 ::std::option::Option::None
            }
        };
        let callbacks: Vec<_> = self
            .attr_signature_info
            .args
            .iter()
            .filter(|arg| matches!(arg.bindgen_ty, BindgenArgType::CallbackArg))
            .map(|arg| {
                let ty = &arg.ty;
                quote! {
                    <#ty as ::near_sdk::borsh::BorshSchema>::schema_container()
                }
            })
            .collect();
        let callbacks_vec = match self
            .attr_signature_info
            .args
            .iter()
            .filter(|arg| matches!(arg.bindgen_ty, BindgenArgType::CallbackArgVec))
            .last()
        {
            None => {
                quote! {
                    ::std::option::Option::None
                }
            }
            Some(arg) => {
                let ty = &arg.ty;
                quote! {
                    ::std::option::Option::Some(<#ty as ::near_sdk::borsh::BorshSchema>::schema_container())
                }
            }
        };
        let result = match &self.attr_signature_info.returns.original {
            ReturnType::Default => {
                quote! {
                    ::std::option::Option::None
                }
            }
            ReturnType::Type(_, ty) => {
                quote! {
                    ::std::option::Option::Some(<#ty as ::near_sdk::borsh::BorshSchema>::schema_container())
                }
            }
        };

        quote! {
             ::near_sdk::__private::MethodMetadata {
                 name: ::std::string::String::from(#method_name_str),
                 is_view: #is_view,
                 is_init: #is_init,
                 args: #args,
                 callbacks: ::std::vec![#(#callbacks),*],
                 callbacks_vec: #callbacks_vec,
                 result: #result
             }
        }
    }
}
