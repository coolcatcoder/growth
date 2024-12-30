use syn::parse2;

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

pub struct Unload(Punctuated<Ident, Token![,]>);

impl Parse for Unload {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Unload(input.parse_terminated(Ident::parse, Token![,])?))
    }
}

pub fn save_and_load_old(input: DeriveInput) -> syn::Result<TokenStream> {
    let struct_ident = input.ident;
    let struct_ident_lowercase = struct_ident.to_string().to_lowercase();

    let mut unload = None;

    for attribute in input.attrs {
        if let Meta::List(attribute) = attribute.meta {
            match attribute.path.to_token_stream().to_string().as_str() {
                "unload" => unload = Some(attribute.tokens),
                _ => (),
            }

            if unload.is_some() {
                break;
            }
        }
    }

    let mut unload_parameters = vec![];
    let mut unload_despawn = vec![];
    parse2::<Unload>(unload.unwrap_or_default())
        .unwrap()
        .0
        .into_iter()
        .for_each(|unload_ident| {
            let unload_ident_lowercase =
                Ident::new(&unload_ident.to_string().to_lowercase(), Span::call_site());
            unload_parameters
                .push(quote! {#unload_ident_lowercase: Query<Entity, With<#unload_ident>>,});
            unload_despawn.push(unload_ident_lowercase);
        });

    match input.data {
        Data::Enum(data) => {
            return Err(Error::new(
                data.enum_token.span,
                "Enums are not supported for saving.",
            ))
        }
        Data::Union(data) => {
            return Err(Error::new(
                data.union_token.span,
                "Unions are not supported for saving.",
            ))
        }
        Data::Struct(data) => {
            let serialised_struct_ident = Ident::new(
                &format!("Serialised{}", struct_ident.to_string()),
                Span::call_site(),
            );

            Ok(match data.fields {
                Fields::Named(fields) => {
                    let mut idents_of_fields = vec![];
                    let mut serialised_types_of_fields = vec![];

                    let mut entity_serialisations = vec![];
                    let mut entity_deserialisations = vec![];

                    let mut to_serialised = vec![];
                    let mut from_serialised = vec![];

                    fields.named.into_iter().for_each(|field| {
                        let field_ident = field.ident.unwrap();

                        let field_type_as_string = field.ty.to_token_stream().to_string();
                        let field_type = if field_type_as_string == "Entity" {
                            from_serialised.push(quote!{#field_ident,});
                            entity_deserialisations.push(quote!{let #field_ident = deserialise_entity.convert(serialised.#field_ident, &mut commands);});

                            entity_serialisations.push(quote!{let #field_ident = serialise_entity.convert(value.#field_ident);});
                            to_serialised.push(quote!{#field_ident,});

                            quote! {SerialisedEntity}
                        } else {
                            from_serialised.push(quote!{#field_ident: serialised.#field_ident,});
                            to_serialised.push(quote!{#field_ident: value.#field_ident.clone(),});

                            field.ty.to_token_stream()
                        };

                        idents_of_fields.push(field_ident);
                        serialised_types_of_fields.push(field_type);
                    });

                    let serialised_fields = idents_of_fields
                        .iter()
                        .zip(serialised_types_of_fields.iter())
                        .map(|(ident, ty)| {
                            quote! {#ident: #ty,}
                        });

                    quote! {
                        #[derive(Asset, TypePath, Serialize, Deserialize)]
                        pub struct #serialised_struct_ident {
                            entity: SerialisedEntity,
                            #(#serialised_fields)*
                        }

                        impl #struct_ident {
                            /// Auto-generated save system.
                            fn save(
                                values: Query<(Entity, &SaveConfig, &Self)>,
                                mut save: EventReader<Save>,
                                mut serialise_entity: ResMut<SerialiseEntity>,
                            ) {
                                save.read().for_each(|save| {
                                    values.iter().for_each(|(entity, save_config, value)| {
                                        // find the save configs whose paths match the path you want to save
                                        // get or create entity folder at the path
                                        // create component file in it

                                        if save_config.path != save.0 {
                                            return;
                                        }

                                        let entity = serialise_entity.convert(entity);
                                        #(#entity_serialisations)*

                                        let serialised = #serialised_struct_ident {
                                            entity,
                                            #(#to_serialised)*
                                        };

                                        // Each entity should have only 1 of each component, so the file is unique.
                                        Save::to_serialised_entity(
                                            &serialised,
                                            entity,
                                            &save_config.path,
                                            #struct_ident_lowercase,
                                        );
                                    });
                                });
                            }

                            fn load(
                                mut load: EventReader<Load>,
                                mut folders: ResMut<Assets<LoadedFolder>>,
                                mut serialised: ResMut<Assets<#serialised_struct_ident>>,
                                asset_server: Res<AssetServer>,
                                mut folder_handle: Local<Option<Handle<LoadedFolder>>>,
                                mut commands: Commands,
                                mut deserialise_entity: ResMut<DeserialiseEntity>,

                                values: Query<Entity, With<Self>>,
                                #(#unload_parameters)*
                            ) {
                                if let Some(handle) = folder_handle.as_ref() {
                                    let folder = some_or_return!(folders.get_mut(handle));

                                    if folder.handles.is_empty() {
                                        *folder_handle = None;
                                        info!("Finished loading folder.");
                                    } else {
                                        // This while loop removes handles as they load.
                                        let mut index = folder.handles.len();
                                        while index != 0 {
                                            index -= 1;
                                            let Ok(line_id) = folder.handles[index].id().try_typed::<#serialised_struct_ident>() else {
                                                // TODO: We are silently ignoring errors. This is because we have to load the whole folder.
                                                // See https://github.com/bevyengine/bevy/issues/2291 for more info.

                                                // debugging
                                                //info!(#struct_ident_lowercase);

                                                // It is important to not remove the handle, because other load systems may be wanting it.
                                                //folder.handles.swap_remove(index);

                                                continue;
                                            };

                                            if let Some(serialised) = serialised.remove(line_id) {
                                                // Iterating backwards, so this is fine.
                                                folder.handles.swap_remove(index);

                                                #(#entity_deserialisations)*

                                                let unserialised = #struct_ident {
                                                    #(#from_serialised)*
                                                };

                                                let entity = deserialise_entity.convert(serialised.entity, &mut commands);
                                                commands.entity(entity).insert(unserialised);

                                                info!("Loaded a file!");
                                            }
                                        }
                                    }
                                } else {
                                    load.read().for_each(|load| {


                                        // Old below.

                                        *folder_handle = Some(asset_server.load_folder(Path::new("./saves").join(path)));

                                        values.despawn_all(&mut commands);
                                        #(#unload_despawn.despawn_all(&mut commands);)*

                                        info!("Loading folder!");
                                    });
                                }
                            }
                        }
                    }
                }
                _ => todo!(),
            })
        }
    }
}
