use crate::prelude::*;

pub fn save_and_load(input: DeriveInput) -> syn::Result<TokenStream> {
    let struct_ident = input.ident;
    let struct_ident_string_lowercase = struct_ident.to_string().to_lowercase();
    let serialised_struct_ident = Ident::new(
        &format!("Serialised{}", struct_ident.to_string()),
        Span::call_site(),
    );

    let file_extension = format!("{}.json", struct_ident_string_lowercase);

    let Data::Struct(data) = input.data else {
        todo!();
    };

    let mut serialised_fields = vec![];
    let mut entity_serialisations = vec![];
    let mut entity_deserialisations = vec![];
    let mut to_serialised = vec![];
    let mut from_serialised = vec![];

    data.fields.into_iter().for_each(|field| {
        let field_ident = field.ident.unwrap();
        let field_type = field.ty;

        let field_type_as_string = field_type.to_token_stream().to_string();

        if field_type_as_string == "Entity" {
            serialised_fields.push(quote! {#field_ident: SerialisedEntity,});
            entity_deserialisations.push(quote!{let #field_ident = deserialise_entity.convert(serialised.#field_ident, commands);});
            entity_serialisations
                .push(quote! {let #field_ident = serialise_entity.convert(self.#field_ident);});
            to_serialised.push(quote! {#field_ident,});
            from_serialised.push(quote!{#field_ident,});
        } else {
            serialised_fields.push(quote! {#field_ident: #field_type,});
            to_serialised.push(quote! {#field_ident: self.#field_ident.clone(),});
            from_serialised.push(quote! {#field_ident: serialised.#field_ident.clone(),});
        }
    });

    Ok(quote! {
        #[derive(Asset, TypePath, Serialize, Deserialize)]
        pub struct #serialised_struct_ident {
            #(#serialised_fields)*
        }

        impl SaveAndLoad for #struct_ident {
            type Serialised = #serialised_struct_ident;

            fn serialise(&self, serialise_entity: &mut SerialiseEntity) -> Self::Serialised {
                #(#entity_serialisations)*

                Self::Serialised {
                    #(#to_serialised)*
                }
            }

            fn deserialise(serialised: &Self::Serialised, deserialise_entity: &mut DeserialiseEntity, commands: &mut Commands) -> Self {
                #(#entity_deserialisations)*

                Self {
                    #(#from_serialised)*
                }
            }

            const STRUCT_IDENT_LOWERCASE: &str = #struct_ident_string_lowercase;
            const FILE_EXTENSION: &str = #file_extension;
        }
    })
}
