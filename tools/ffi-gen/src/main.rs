use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    Attribute, Expr, ExprLit, Fields, File, Ident, Item, ItemEnum, ItemStruct, Lit, Meta, Type,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ComponentKind {
    Container,
    Leaf,
}

struct ComponentSpec {
    function_stem: String,
    variant: Ident,
    kind: ComponentKind,
}

#[derive(Default)]
struct ModuleMetadata {
    kind: Option<ComponentKind>,
    skip: bool,
    variant: Option<Ident>,
}

#[derive(Clone)]
enum VariantPayload {
    Unit,
    Tuple(Type),
}

struct FieldPlan {
    argument_name: Ident,
    ffi_type: Type,
    conversion: Option<TokenStream>,
    field_name: Ident,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);

    let components_path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| invalid_input("components/mod.rsが指定されていません"))?;

    let runtime_node_path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| invalid_input("runtime/node.rsが指定されていません"))?;

    let output_path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| invalid_input("生成先が指定されていません"))?;

    if arguments.next().is_some() {
        return Err(invalid_input("引数が多すぎます").into());
    }

    let components_file = parse_file(&components_path)?;
    let runtime_file = parse_file(&runtime_node_path)?;

    let variants = collect_variants(&runtime_file)?;
    let structs = collect_structs(&runtime_file);
    let specs = collect_component_specs(&components_file, &variants)?;
    let generated = generate(&specs, &variants, &structs)?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    write_if_changed(&output_path, &generated)?;

    Ok(())
}

fn parse_file(path: &Path) -> Result<File, Box<dyn Error>> {
    let source = fs::read_to_string(path)?;

    syn::parse_file(&source).map_err(|error| {
        invalid_input(format!("{}の解析に失敗しました: {error}", path.display(),)).into()
    })
}

fn collect_variants(file: &File) -> Result<HashMap<String, VariantPayload>, Box<dyn Error>> {
    let view_node_kind = file
        .items
        .iter()
        .find_map(|item| match item {
            Item::Enum(item_enum) if item_enum.ident.to_string() == "ViewNodeKind" => {
                Some(item_enum)
            }
            _ => None,
        })
        .ok_or_else(|| invalid_input("ViewNodeKindが見つかりません"))?;

    collect_enum_variants(view_node_kind)
}

fn collect_enum_variants(
    item_enum: &ItemEnum,
) -> Result<HashMap<String, VariantPayload>, Box<dyn Error>> {
    let mut variants = HashMap::new();

    for variant in &item_enum.variants {
        let payload = match &variant.fields {
            Fields::Unit => VariantPayload::Unit,

            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field = fields
                    .unnamed
                    .first()
                    .ok_or_else(|| invalid_input("variant payloadが見つかりません"))?;

                VariantPayload::Tuple(field.ty.clone())
            }

            _ => {
                return Err(invalid_input(format!(
                    "ViewNodeKind::{}はunit variantまたは単一tuple variantである必要があります",
                    variant.ident,
                ))
                .into());
            }
        };

        variants.insert(variant.ident.to_string(), payload);
    }

    Ok(variants)
}

fn collect_structs(file: &File) -> HashMap<String, ItemStruct> {
    file.items
        .iter()
        .filter_map(|item| match item {
            Item::Struct(item_struct) => Some((item_struct.ident.to_string(), item_struct.clone())),
            _ => None,
        })
        .collect()
}

fn collect_component_specs(
    file: &File,
    variants: &HashMap<String, VariantPayload>,
) -> Result<Vec<ComponentSpec>, Box<dyn Error>> {
    let mut specs = Vec::new();
    let mut seen_variants = HashSet::new();

    for item in &file.items {
        let Item::Mod(item_mod) = item else {
            continue;
        };

        let metadata = parse_module_metadata(&item_mod.attrs)?;

        if metadata.skip {
            continue;
        }

        let module_name = item_mod.ident.to_string();

        let Some(variant) = resolve_variant(&module_name, metadata.variant, variants)? else {
            /*
             * ViewNodeKindに対応するvariantがない合成UIコンポーネントは、
             * FFI Runtimeの登録対象ではありません。
             */
            continue;
        };

        if !seen_variants.insert(variant.to_string()) {
            return Err(invalid_input(format!(
                "ViewNodeKind::{variant}が複数のcomponent moduleから登録されています",
            ))
            .into());
        }

        specs.push(ComponentSpec {
            function_stem: module_name,
            variant,
            kind: metadata.kind.unwrap_or(ComponentKind::Leaf),
        });
    }

    for directive in doc_lines(&file.attrs) {
        let Some(rest) = directive.strip_prefix("@ffi synthetic ") else {
            continue;
        };

        let parts: Vec<_> = rest.split_whitespace().collect();

        if parts.len() != 2 {
            return Err(
                invalid_input(format!("不正なsynthetic directiveです: {directive}",)).into(),
            );
        }

        let variant: Ident = syn::parse_str(parts[0])
            .map_err(|_| invalid_input(format!("不正なsynthetic variant名です: {}", parts[0],)))?;

        let kind = parse_kind(parts[1])?;

        if !variants.contains_key(&variant.to_string()) {
            return Err(invalid_input(format!(
                "synthetic componentのViewNodeKind::{variant}が見つかりません",
            ))
            .into());
        }

        if !seen_variants.insert(variant.to_string()) {
            return Err(
                invalid_input(format!("ViewNodeKind::{variant}が重複登録されています",)).into(),
            );
        }

        specs.push(ComponentSpec {
            function_stem: to_snake_case(&variant.to_string()),
            variant,
            kind,
        });
    }

    Ok(specs)
}

fn resolve_variant(
    module_name: &str,
    explicit_variant: Option<Ident>,
    variants: &HashMap<String, VariantPayload>,
) -> Result<Option<Ident>, Box<dyn Error>> {
    if let Some(variant) = explicit_variant {
        if !variants.contains_key(&variant.to_string()) {
            return Err(invalid_input(format!("ViewNodeKind::{variant}が見つかりません",)).into());
        }

        return Ok(Some(variant));
    }

    let normalized_module = normalize_name(module_name);

    let matches: Vec<_> = variants
        .keys()
        .filter(|variant| normalize_name(variant) == normalized_module)
        .collect();

    match matches.as_slice() {
        [] => Ok(None),

        [variant] => {
            let ident: Ident = syn::parse_str(variant).map_err(|_| {
                invalid_input(format!("不正なViewNodeKind variant名です: {variant}",))
            })?;

            Ok(Some(ident))
        }

        _ => Err(invalid_input(format!(
            "module `{module_name}`に対応するViewNodeKind variantが複数あります",
        ))
        .into()),
    }
}

fn parse_module_metadata(attrs: &[Attribute]) -> Result<ModuleMetadata, Box<dyn Error>> {
    let mut metadata = ModuleMetadata::default();

    for directive in doc_lines(attrs) {
        let Some(rest) = directive.strip_prefix("@ffi ") else {
            continue;
        };

        if rest == "container" {
            metadata.kind = Some(ComponentKind::Container);
        } else if rest == "leaf" {
            metadata.kind = Some(ComponentKind::Leaf);
        } else if rest == "skip" {
            metadata.skip = true;
        } else if let Some(variant) = rest.strip_prefix("variant ") {
            let variant: Ident = syn::parse_str(variant.trim()).map_err(|_| {
                invalid_input(format!("不正なFFI variant名です: {}", variant.trim(),))
            })?;

            metadata.variant = Some(variant);
        } else {
            return Err(invalid_input(format!("不明なFFI directiveです: {directive}",)).into());
        }
    }

    Ok(metadata)
}

fn doc_lines(attrs: &[Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter_map(|attribute| {
            if !attribute.path().is_ident("doc") {
                return None;
            }

            let Meta::NameValue(name_value) = &attribute.meta else {
                return None;
            };

            let Expr::Lit(ExprLit {
                lit: Lit::Str(value),
                ..
            }) = &name_value.value
            else {
                return None;
            };

            Some(value.value().trim().to_owned())
        })
        .collect()
}

fn parse_kind(value: &str) -> Result<ComponentKind, Box<dyn Error>> {
    match value {
        "container" => Ok(ComponentKind::Container),
        "leaf" => Ok(ComponentKind::Leaf),

        _ => Err(invalid_input(format!(
            "component kindはcontainerまたはleafである必要があります: {value}",
        ))
        .into()),
    }
}

fn generate(
    specs: &[ComponentSpec],
    variants: &HashMap<String, VariantPayload>,
    structs: &HashMap<String, ItemStruct>,
) -> Result<String, Box<dyn Error>> {
    let mut functions = Vec::new();

    for spec in specs {
        let payload = variants.get(&spec.variant.to_string()).ok_or_else(|| {
            invalid_input(format!("ViewNodeKind::{}が見つかりません", spec.variant,))
        })?;

        functions.push(generate_component(spec, payload, structs)?);
    }

    let body = quote! {
        use super::*;

        #(#functions)*
    };

    Ok(format!(
        "// @generated by tools/ffi-gen. Do not edit manually.\n\n{body}\n",
    ))
}

fn generate_component(
    spec: &ComponentSpec,
    payload: &VariantPayload,
    structs: &HashMap<String, ItemStruct>,
) -> Result<TokenStream, Box<dyn Error>> {
    let function_prefix = match spec.kind {
        ComponentKind::Container => "vk_begin_",
        ComponentKind::Leaf => "vk_push_",
    };

    let function_name = format_ident!("{}{}", function_prefix, spec.function_stem,);

    let variant = &spec.variant;

    let builder_method = match spec.kind {
        ComponentKind::Container => format_ident!("begin"),
        ComponentKind::Leaf => format_ident!("leaf"),
    };

    let (arguments, conversions, node_expression) =
        generate_node_expression(variant, payload, structs)?;

    Ok(quote! {
        #[unsafe(no_mangle)]
        #[allow(clippy::too_many_arguments)]
        pub extern "C" fn #function_name(
            runtime: *mut VkRuntime,
            node_id: u64,
            #(#arguments,)*
        ) -> i32 {
            ffi_status(|| {
                #(#conversions)*

                let runtime = runtime_mut(runtime)?;
                let builder = active_builder(runtime)?;
                let node = #node_expression;

                builder.#builder_method(
                    crate::runtime::ViewNode::new(
                        crate::runtime::NodeId(node_id),
                        node,
                    ),
                );

                Ok(())
            })
        }
    })
}

fn generate_node_expression(
    variant: &Ident,
    payload: &VariantPayload,
    structs: &HashMap<String, ItemStruct>,
) -> Result<(Vec<TokenStream>, Vec<TokenStream>, TokenStream), Box<dyn Error>> {
    match payload {
        VariantPayload::Unit => Ok((
            Vec::new(),
            Vec::new(),
            quote!(
                crate::runtime::ViewNodeKind::#variant
            ),
        )),

        VariantPayload::Tuple(ty) => {
            let node_type = last_type_ident(ty).ok_or_else(|| {
                invalid_input(format!(
                    "ViewNodeKind::{variant}のpayload型を解決できません",
                ))
            })?;

            /*
             * 現行ABIとの互換性を保つ専用adapterです。
             */
            if node_type == "RectangleNode" {
                return Ok((
                    vec![quote!(style: VkRectangleStyle)],
                    vec![quote!(
                        let properties =
                            decode_rectangle_style(style)?;
                    )],
                    quote!(
                        crate::runtime::ViewNodeKind::#variant(
                            properties
                        )
                    ),
                ));
            }

            if node_type == "TextNode" {
                return Ok(generate_text_node(variant));
            }

            if node_type == "ButtonNode" {
                return Ok(generate_button_node(variant));
            }

            let item_struct = structs.get(&node_type).ok_or_else(|| {
                invalid_input(format!(
                    "ViewNodeKind::{variant}のpayload struct \
                         `{node_type}`が見つかりません",
                ))
            })?;

            generate_struct_node(variant, &node_type, item_struct)
        }
    }
}

fn generate_text_node(variant: &Ident) -> (Vec<TokenStream>, Vec<TokenStream>, TokenStream) {
    let arguments = vec![
        quote!(content: VkString),
        quote!(font_size: f32),
        quote!(line_height: f32),
        quote!(weight: u16),
        quote!(alignment: u32),
        quote!(color: u32),
    ];

    let conversions = vec![
        quote!(
            let content = copy_string(content)?;
        ),
        quote!(
            let font_size =
                finite_or_default(font_size, 16.0);
        ),
        quote!(
            let line_height =
                finite_or_default(line_height, 24.0);
        ),
        quote!(
            let alignment =
                decode_text_alignment(alignment)?;
        ),
        quote!(
            let color = decode_text_color(color)?;
        ),
    ];

    let node = quote! {
        crate::runtime::ViewNodeKind::#variant(
            crate::runtime::TextNode {
                content,
                font_family:
                    String::from("Noto Sans JP"),
                font_size,
                line_height,
                weight,
                alignment,
                color,
            }
        )
    };

    (arguments, conversions, node)
}

fn generate_button_node(variant: &Ident) -> (Vec<TokenStream>, Vec<TokenStream>, TokenStream) {
    let arguments = vec![
        quote!(title: VkString),
        quote!(color: u32),
        quote!(radius: f32),
        quote!(action_id: u64),
    ];

    let conversions = vec![
        quote!(
            let title = copy_string(title)?;
        ),
        quote!(
            let color =
                decode_button_color(color)?;
        ),
        quote!(
            let radius =
                sanitize_length(radius);
        ),
        quote! {
            let action = if action_id == 0 {
                None
            } else {
                Some(
                    crate::runtime::ActionId(
                        action_id,
                    ),
                )
            };
        },
    ];

    let node = quote! {
        crate::runtime::ViewNodeKind::#variant(
            crate::runtime::ButtonNode {
                title,
                color,
                radius,
                action,
            }
        )
    };

    (arguments, conversions, node)
}

fn generate_struct_node(
    variant: &Ident,
    node_type: &str,
    item_struct: &ItemStruct,
) -> Result<(Vec<TokenStream>, Vec<TokenStream>, TokenStream), Box<dyn Error>> {
    let Fields::Named(fields) = &item_struct.fields else {
        return Err(invalid_input(format!(
            "{node_type}はnamed-field structである必要があります",
        ))
        .into());
    };

    let mut arguments = Vec::new();
    let mut conversions = Vec::new();
    let mut field_names = Vec::new();

    for field in &fields.named {
        let field_name = field
            .ident
            .clone()
            .ok_or_else(|| invalid_input(format!("{node_type}に名前のないfieldがあります",)))?;

        let plan = plan_field(node_type, &field_name, &field.ty)?;

        let argument_name = &plan.argument_name;
        let ffi_type = &plan.ffi_type;

        arguments.push(quote!(#argument_name: #ffi_type));

        if let Some(conversion) = plan.conversion {
            conversions.push(conversion);
        }

        field_names.push(plan.field_name);
    }

    let node_type_ident = Ident::new(node_type, Span::call_site());

    let node = quote! {
        crate::runtime::ViewNodeKind::#variant(
            crate::runtime::#node_type_ident {
                #(#field_names,)*
            }
        )
    };

    Ok((arguments, conversions, node))
}

fn plan_field(node_type: &str, field_name: &Ident, ty: &Type) -> Result<FieldPlan, Box<dyn Error>> {
    let key = normalized_type(ty);
    let field = field_name.clone();

    let plan = match key.as_str() {
        "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "f64" => FieldPlan {
            argument_name: field.clone(),
            ffi_type: ty.clone(),
            conversion: None,
            field_name: field,
        },

        "f32" => {
            let conversion = if node_type == "PaddingNode" {
                Some(quote!(
                    let #field =
                        sanitize_length(#field);
                ))
            } else {
                None
            };

            FieldPlan {
                argument_name: field.clone(),
                ffi_type: ty.clone(),
                conversion,
                field_name: field,
            }
        }

        "bool" => FieldPlan {
            argument_name: field.clone(),
            ffi_type: syn::parse_quote!(u8),
            conversion: Some(quote!(
                let #field = #field != 0;
            )),
            field_name: field,
        },

        "String" => FieldPlan {
            argument_name: field.clone(),
            ffi_type: syn::parse_quote!(VkString),
            conversion: Some(quote!(
                let #field =
                    copy_string(#field)?;
            )),
            field_name: field,
        },

        "StackGap" => converted_field(field, syn::parse_quote!(u32), "decode_stack_gap"),

        "StackAlignment" => {
            converted_field(field, syn::parse_quote!(u32), "decode_stack_alignment")
        }

        "StackDistribution" => {
            converted_field(field, syn::parse_quote!(u32), "decode_stack_distribution")
        }

        "ZStackAlignment" => {
            converted_field(field, syn::parse_quote!(u32), "decode_zstack_alignment")
        }

        "LayoutLength" => {
            converted_field(field, syn::parse_quote!(VkLength), "decode_layout_length")
        }

        "TextAlignment" => converted_field(field, syn::parse_quote!(u32), "decode_text_alignment"),

        "ButtonColor" => converted_field(field, syn::parse_quote!(u32), "decode_button_color"),

        "Color" => infallible_converted_field(field, syn::parse_quote!(VkColor), "decode_color"),

        "Option<ActionId>" => {
            let argument_name = format_ident!("{}_id", field);

            FieldPlan {
                argument_name: argument_name.clone(),
                ffi_type: syn::parse_quote!(u64),
                conversion: Some(quote! {
                    let #field =
                        if #argument_name == 0 {
                            None
                        } else {
                            Some(
                                crate::runtime::ActionId(
                                    #argument_name,
                                ),
                            )
                        };
                }),
                field_name: field,
            }
        }

        _ => {
            return Err(invalid_input(format!(
                "{node_type}::{field_name}の型 `{key}` をC ABIへ変換できません",
            ))
            .into());
        }
    };

    Ok(plan)
}

fn converted_field(field: Ident, ffi_type: Type, decoder: &str) -> FieldPlan {
    let decoder = Ident::new(decoder, Span::call_site());

    FieldPlan {
        argument_name: field.clone(),
        ffi_type,
        conversion: Some(quote!(
            let #field = #decoder(#field)?;
        )),
        field_name: field,
    }
}

fn infallible_converted_field(field: Ident, ffi_type: Type, decoder: &str) -> FieldPlan {
    let decoder = Ident::new(decoder, Span::call_site());

    FieldPlan {
        argument_name: field.clone(),
        ffi_type,
        conversion: Some(quote!(
            let #field = #decoder(#field);
        )),
        field_name: field,
    }
}

fn normalized_type(ty: &Type) -> String {
    quote!(#ty).to_string().replace(' ', "")
}

fn last_type_ident(ty: &Type) -> Option<String> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    type_path
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

fn to_snake_case(value: &str) -> String {
    let mut output = String::new();
    let characters: Vec<_> = value.chars().collect();

    for (index, character) in characters.iter().copied().enumerate() {
        if character.is_ascii_uppercase() {
            let previous_is_lower = index > 0 && characters[index - 1].is_ascii_lowercase();

            let next_is_lower = characters
                .get(index + 1)
                .is_some_and(|next| next.is_ascii_lowercase());

            if index > 0 && (previous_is_lower || next_is_lower) {
                output.push('_');
            }

            output.push(character.to_ascii_lowercase());
        } else {
            output.push(character);
        }
    }

    output
}

fn write_if_changed(path: &Path, content: &str) -> io::Result<()> {
    if fs::read_to_string(path).is_ok_and(|existing| existing == content) {
        return Ok(());
    }

    fs::write(path, content)
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}
