use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, File, Ident, Item, Result as SynResult, Token, Type, parenthesized};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComponentKind {
    Container,
    Leaf,
}

impl Parse for ComponentKind {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let kind: Ident = input.parse()?;

        match kind.to_string().as_str() {
            "container" => Ok(Self::Container),
            "leaf" => Ok(Self::Leaf),

            _ => Err(syn::Error::new(
                kind.span(),
                "component kind must be `container` or `leaf`",
            )),
        }
    }
}

#[derive(Debug)]
struct Argument {
    name: Ident,
    ty: Type,

    binding: Option<Ident>,
    conversion: Option<Expr>,
}

impl Parse for Argument {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let name: Ident = input.parse()?;

        input.parse::<Token![:]>()?;

        let ty: Type = input.parse()?;

        let (binding, conversion) = if input.peek(Token![=>]) {
            input.parse::<Token![=>]>()?;

            let binding: Ident = input.parse()?;

            input.parse::<Token![=]>()?;

            let conversion: Expr = input.parse()?;

            (Some(binding), Some(conversion))
        } else {
            (None, None)
        };

        Ok(Self {
            name,
            ty,
            binding,
            conversion,
        })
    }
}

#[derive(Debug)]
struct Component {
    kind: ComponentKind,
    name: Ident,

    arguments: Vec<Argument>,

    node: Expr,
}

impl Parse for Component {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let kind: ComponentKind = input.parse()?;

        let name: Ident = input.parse()?;

        let arguments_content;

        parenthesized!(arguments_content in input);

        let arguments = Punctuated::<Argument, Token![,]>::parse_terminated(&arguments_content)?
            .into_iter()
            .collect();

        input.parse::<Token![=>]>()?;

        let node: Expr = input.parse()?;

        input.parse::<Token![;]>()?;

        Ok(Self {
            kind,
            name,
            arguments,
            node,
        })
    }
}

#[derive(Debug)]
struct ComponentManifest {
    components: Vec<Component>,
}

impl Parse for ComponentManifest {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let mut components = Vec::new();

        while !input.is_empty() {
            components.push(input.parse()?);
        }

        Ok(Self { components })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);

    let input = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| invalid_input("components/mod.rsが指定されていません"))?;

    let output = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| invalid_input("生成先が指定されていません"))?;

    if arguments.next().is_some() {
        return Err(invalid_input("引数が多すぎます").into());
    }

    let source = fs::read_to_string(&input)?;

    let file: File = syn::parse_file(&source)?;

    let tokens = find_ffi_components_macro(&file)?;

    let manifest: ComponentManifest = syn::parse2(tokens)?;

    validate_manifest(&manifest)?;

    let generated = generate(&manifest);

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    write_if_changed(&output, &generated)?;

    Ok(())
}

fn find_ffi_components_macro(file: &File) -> Result<TokenStream, Box<dyn Error>> {
    for item in &file.items {
        let Item::Macro(item_macro) = item else {
            continue;
        };

        let Some(last_segment) = item_macro.mac.path.segments.last() else {
            continue;
        };

        if last_segment.ident == "ffi_components" {
            return Ok(item_macro.mac.tokens.clone());
        }
    }

    Err(invalid_input("ffi_components!が見つかりません").into())
}

fn validate_manifest(manifest: &ComponentManifest) -> Result<(), Box<dyn Error>> {
    let mut names = HashSet::new();

    for component in &manifest.components {
        let name = component.name.to_string();

        if !name.starts_with("vk_") {
            return Err(invalid_input(format!("FFI関数名はvk_で始めてください: {name}",)).into());
        }

        if !names.insert(name.clone()) {
            return Err(invalid_input(format!("FFI関数が重複しています: {name}",)).into());
        }

        let mut argument_names = HashSet::new();

        for argument in &component.arguments {
            let argument_name = argument.name.to_string();

            if !argument_names.insert(argument_name.clone()) {
                return Err(invalid_input(format!(
                    "{name}の引数が重複しています: {argument_name}",
                ))
                .into());
            }

            if argument.binding.is_some() != argument.conversion.is_some() {
                return Err(invalid_input(format!(
                    "{name}::{argument_name}の変換定義が不完全です",
                ))
                .into());
            }
        }
    }

    Ok(())
}

fn generate(manifest: &ComponentManifest) -> String {
    let functions = manifest.components.iter().map(generate_component);

    let output = quote! {
        // @generated by tools/ffi-gen.
        // Do not edit manually.

        use super::*;

        #(#functions)*
    };

    output.to_string()
}

fn generate_component(component: &Component) -> TokenStream {
    let name = &component.name;

    let argument_names = component.arguments.iter().map(|argument| &argument.name);

    let argument_types = component.arguments.iter().map(|argument| &argument.ty);

    let conversions = component.arguments.iter().filter_map(|argument| {
        let binding = argument.binding.as_ref()?;

        let conversion = argument.conversion.as_ref()?;

        Some(quote! {
            let #binding = #conversion;
        })
    });

    let node = &component.node;

    let builder_method = match component.kind {
        ComponentKind::Container => Ident::new("begin", component.name.span()),

        ComponentKind::Leaf => Ident::new("leaf", component.name.span()),
    };

    quote! {
        #[unsafe(no_mangle)]
        #[allow(clippy::too_many_arguments)]
        pub extern "C" fn #name(
            runtime: *mut VkRuntime,
            node_id: u64,
            #(
                #argument_names:
                    #argument_types,
            )*
        ) -> i32 {
            ffi_status(|| {
                #(#conversions)*

                let runtime =
                    runtime_mut(runtime)?;

                let builder =
                    active_builder(runtime)?;

                let node = #node;

                builder.#builder_method(
                    crate::runtime::ViewNode::new(
                        crate::runtime::NodeId(
                            node_id,
                        ),
                        node,
                    ),
                );

                Ok(())
            })
        }
    }
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
