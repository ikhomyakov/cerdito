extern crate proc_macro;
use quote::quote;

#[proc_macro_derive(Encode)]
pub fn encode_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();
    let body = match ast.data {
        syn::Data::Struct(ref data) => generate_encode_for_struct(data, &name),
        syn::Data::Enum(ref data) => generate_encode_for_enum(data, &name),
        syn::Data::Union(_) => unimplemented!("Unions are not supported"),
    };
    let expanded = quote! {
        #[automatically_derived]
        impl #impl_generics ::cerdito::Encode for #name #type_generics #where_clause {
            #[_async] fn encode<__CerditoEncoderTypeParam: ::cerdito::Encoder>(
                &self,
                encoder: &mut __CerditoEncoderTypeParam
            ) -> Result<(), __CerditoEncoderTypeParam::Error> {
                #body
            }
        }
    };
    proc_macro::TokenStream::from(expanded)
}

#[proc_macro_derive(Decode)]
pub fn decode_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();
    let body = match ast.data {
        syn::Data::Struct(ref data) => generate_decode_for_struct(data, &name),
        syn::Data::Enum(ref data) => generate_decode_for_enum(data, &name),
        syn::Data::Union(_) => unimplemented!("Unions are not supported"),
    };
    let expanded = quote! {
        #[automatically_derived]
        impl #impl_generics ::cerdito::Decode for #name #type_generics #where_clause {
            #[_async] fn decode<__CerditoDecoderTypeParam: ::cerdito::Decoder>(
                decoder: &mut __CerditoDecoderTypeParam
            ) -> Result<Self, __CerditoDecoderTypeParam::Error> {
                #body
            }
        }
    };
    proc_macro::TokenStream::from(expanded)
}

fn get_fields(fields: &syn::Fields) -> Vec<(usize, proc_macro2::Ident, String, syn::Type)> {
    match fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let default_name = format!("field_{}", i);
                let field_ident = f
                    .ident
                    .clone()
                    .or(Some(proc_macro2::Ident::new(
                        &default_name,
                        proc_macro2::Span::call_site(),
                    )))
                    .unwrap();
                let field_name = field_ident.to_string();
                (i, field_ident, field_name, f.ty.clone())
            })
            .collect(),
        syn::Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let default_name = format!("field_{}", i);
                let field_ident = f
                    .ident
                    .clone()
                    .or(Some(proc_macro2::Ident::new(
                        &default_name,
                        proc_macro2::Span::call_site(),
                    )))
                    .unwrap();
                let field_name = field_ident.to_string();
                (i, field_ident, field_name, f.ty.clone())
            })
            .collect(),
        syn::Fields::Unit => vec![],
    }
}

fn generate_encode_for_struct(
    data: &syn::DataStruct,
    name: &proc_macro2::Ident,
) -> proc_macro2::TokenStream {
    let name_str = name.to_string();
    let fields = get_fields(&data.fields);
    let field_idents: Vec<_> = fields
        .iter()
        .map(|(_, ident, _, _)| ident.clone())
        .collect();
    let field_codes: Vec<_> = fields
        .iter()
        .map(|(i, field_ident, field_name, _)| {
            quote! {
                _await!(encoder.encode_elem_begin(#i, Some(#field_name)))?;
                _await!(#field_ident.encode(encoder))?;
                _await!(encoder.encode_elem_end())?;
            }
        })
        .collect();
    let fields_len = fields.len();
    match &data.fields {
        syn::Fields::Named(_) => quote! {
            _await!(encoder.encode_struct_begin(#fields_len, Some(#name_str)))?;
            let Self { #(#field_idents),* } = self;
            #(#field_codes)*
            _await!(encoder.encode_struct_end())?;
            Ok(())
        },
        syn::Fields::Unnamed(_) => quote! {
            _await!(encoder.encode_struct_begin(#fields_len, Some(#name_str)))?;
            let Self( #(#field_idents),* ) = self;
            #(#field_codes)*
            _await!(encoder.encode_struct_end())?;
            Ok(())
        },
        syn::Fields::Unit => quote! {
            _await!(encoder.encode_struct_begin(#fields_len, Some(#name_str)))?;
            _await!(encoder.encode_struct_end())?;
            Ok(())
        },
    }
}

fn generate_decode_for_struct(
    data: &syn::DataStruct,
    name: &proc_macro2::Ident,
) -> proc_macro2::TokenStream {
    let name_str = name.to_string();
    let fields = get_fields(&data.fields);
    let field_idents: Vec<_> = fields
        .iter()
        .map(|(_, ident, _, _)| ident.clone())
        .collect();
    let field_codes: Vec<_> = fields
        .iter()
        .map(|(i, field_ident, field_name, field_type)| {
            quote! {
                _await!(decoder.decode_elem_begin(#i, Some(#field_name)))?;
                let #field_ident = if #i < __cerdito_len {
                    _await!(<#field_type as ::cerdito::Decode>::decode(decoder))?
                } else { // new program, old data
                    <#field_type>::default() // TODO: Or fail if Default isn't implemented?
                };
                _await!(decoder.decode_elem_end())?;
            }
        })
        .collect();
    let fields_len = fields.len();

    let compat = quote! {
        // old program, new data
        if __cerdito_len > #fields_len {
            _await!(decoder.decode_skip(__cerdito_len - #fields_len))?;
        }
    };

    match &data.fields {
        syn::Fields::Named(_) => quote! {
            let __cerdito_len = _await!(decoder.decode_struct_begin(#fields_len, Some(#name_str)))?;
            #(#field_codes)*
            #compat
            _await!(decoder.decode_struct_end())?;
            Ok(Self { #(#field_idents),* })
        },
        syn::Fields::Unnamed(_) => quote! {
            let __cerdito_len = _await!(decoder.decode_struct_begin(#fields_len, Some(#name_str)))?;
            #(#field_codes)*
            #compat
            _await!(decoder.decode_struct_end())?;
            Ok(Self( #(#field_idents),* ))
        },
        syn::Fields::Unit => quote! {
            let __cerdito_len = _await!(decoder.decode_struct_begin(#fields_len, Some(#name_str)))?;
            #compat
            _await!(decoder.decode_struct_end())?;
            Ok(Self)
        },
    }
}

fn generate_tags(data: &syn::DataEnum) -> Vec<proc_macro2::TokenStream> {
    let mut current_expr: Option<proc_macro2::TokenStream> = None;
    let mut current_incr: u32 = 0;
    data.variants
        .iter()
        .map(|v| match &v.discriminant {
            Some((_, expr)) => {
                let e = quote! { #expr };
                current_expr = Some(e.clone());
                current_incr = 1;
                e
            }
            None => match &current_expr {
                Some(expr) => {
                    let e = quote! { (#expr) + #current_incr };
                    current_incr += 1;
                    e
                }
                None => {
                    let e = quote! { #current_incr };
                    current_incr += 1;
                    e
                }
            },
        })
        .collect()
}

fn generate_encode_for_enum(
    data: &syn::DataEnum,
    name: &proc_macro2::Ident,
) -> proc_macro2::TokenStream {
    let name_str = name.to_string();
    let tags = generate_tags(data);
    let variant_codes: Vec<_> = data.variants.iter().zip(tags).enumerate().map(|(i, (v, t))| {
        let variant_name = v.ident.clone();
        let variant_name_str = v.ident.to_string();
        let fields = get_fields(&v.fields);
        let field_idents: Vec<_> = fields
            .iter()
            .map(|(_, ident, _, _)| ident.clone())
            .collect();
        let field_codes: Vec<_> = fields
            .iter()
            .map(|(_i, field_ident, field_name, _field_type)| {
                quote! {
                    _await!(encoder.encode_elem_begin(#i, Some(#field_name)))?;
                    _await!(#field_ident.encode(encoder))?;
                    _await!(encoder.encode_elem_end())?;
                }
            })
            .collect();
        let fields_len = fields.len();
        match &v.fields {
            syn::Fields::Named(_) => quote! {
                Self::#variant_name { #(#field_idents),* } => {
                    let __cerdito_enum_tag: u32 = (#t).try_into().unwrap(); //TODO: error
                    _await!(encoder.encode_enum_begin(__cerdito_enum_tag, 1, #name_str, #variant_name_str))?;
                    _await!(encoder.encode_struct_begin(#fields_len, None))?;
                    #(#field_codes)*
                    _await!(encoder.encode_struct_end())?;
                    _await!(encoder.encode_enum_end())?;
                }
            },
            syn::Fields::Unnamed(_) => quote! {
                Self::#variant_name(#(#field_idents),*) => {
                    let __cerdito_enum_tag: u32 = (#t).try_into().unwrap(); //TODO: error
                    _await!(encoder.encode_enum_begin(__cerdito_enum_tag, 1, #name_str, #variant_name_str))?;
                    _await!(encoder.encode_struct_begin(#fields_len, None))?;
                    #(#field_codes)*
                    _await!(encoder.encode_struct_end())?;
                    _await!(encoder.encode_enum_end())?;
                }
            },
            syn::Fields::Unit => quote! {
                Self::#variant_name => {
                    let __cerdito_enum_tag: u32 = (#t).try_into().unwrap(); //TODO: error
                    _await!(encoder.encode_enum_begin(__cerdito_enum_tag, 0, #name_str, #variant_name_str))?;
                    _await!(encoder.encode_enum_end())?;
                }
            },
        }
    }).collect();

    quote! {
        match self {
            #(#variant_codes)*
        }
        Ok(())
    }
}

fn generate_decode_for_enum(
    data: &syn::DataEnum,
    name: &proc_macro2::Ident,
) -> proc_macro2::TokenStream {
    let name_str = name.to_string();
    let tags = generate_tags(data);
    let variant_codes: Vec<_> = data
        .variants
        .iter()
        .zip(tags)
        .enumerate()
        .map(|(_i, (v, t))| {
            let variant_name = v.ident.clone();
            let fields = get_fields(&v.fields);
            let field_idents: Vec<_> = fields
                .iter()
                .map(|(_, ident, _, _)| ident.clone())
                .collect();
            let field_codes: Vec<_> = fields
                .iter()
                .map(|(i, field_ident, field_name, field_type)| {
                    quote! {
                        _await!(decoder.decode_elem_begin(#i, Some(#field_name)))?;
                        let #field_ident = if #i < __cerdito_len {
                            _await!(<#field_type as ::cerdito::Decode>::decode(decoder))?
                        } else { // new program, old data
                            <#field_type>::default()
                        };
                        _await!(decoder.decode_elem_end())?;
                    }
                })
                .collect();

            let field_defaults: Vec<_> = fields
                .iter()
                .map(|(_i, field_ident, _field_name, field_type)| {
                    quote! {
                        let #field_ident = <#field_type>::default();
                    }
                })
                .collect();

            let fields_len = fields.len();

            let compat = quote! {
                // old program, new data
                if __cerdito_len > #fields_len {
                    _await!(decoder.decode_skip(__cerdito_len - #fields_len))?;
                }
            };

            match &v.fields {
                syn::Fields::Named(_) => quote! {
                    #t => {
                        match __cerdito_enum_len {
                            0 => {
                                #(#field_defaults)*
                                Self::#variant_name { #(#field_idents),* }
                            }
                            1 => {
                                let __cerdito_len = _await!(decoder.decode_struct_begin(#fields_len, None))?;
                                #(#field_codes)*
                                #compat
                                _await!(decoder.decode_struct_end())?;
                                Self::#variant_name { #(#field_idents),* }
                            }
                            _ => unreachable!(),
                        }
                    }
                },
                syn::Fields::Unnamed(_) => quote! {
                    #t => {
                        match __cerdito_enum_len {
                            0 => {
                                #(#field_defaults)*
                                Self::#variant_name(#(#field_idents),*)
                            }
                            1 => {
                                let __cerdito_len = _await!(decoder.decode_struct_begin(#fields_len, None))?;
                                #(#field_codes)*
                                #compat
                                _await!(decoder.decode_struct_end())?;
                                Self::#variant_name(#(#field_idents),*)
                            }
                            _ => unreachable!(),
                        }
                    }
                },
                syn::Fields::Unit => quote! {
                    #t => {
                        match __cerdito_enum_len {
                            0 => {
                                Self::#variant_name
                            }
                            1 => {
                                let __cerdito_len = _await!(decoder.decode_struct_begin(#fields_len, None))?;
                                #compat
                                _await!(decoder.decode_struct_end())?;
                                Self::#variant_name
                            }
                            _ => unreachable!(),
                        }
                    }
                },
            }
        })
        .collect();

    quote! {
        let (__cerdito_enum_tag, __cerdito_enum_len) = _await!(decoder.decode_enum_begin(#name_str))?;
        let __cerdito_enum_value = match __cerdito_enum_tag.try_into().unwrap() { // TODO: error
                #(#variant_codes)*
                _ => panic!("Enum {:?} doesn't support variant {}", #name_str, __cerdito_enum_tag),
        };
        _await!(decoder.decode_enum_end())?;
        Ok(__cerdito_enum_value)
    }
}
