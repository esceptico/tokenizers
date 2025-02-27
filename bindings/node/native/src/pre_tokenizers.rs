extern crate tokenizers as tk;

use crate::extraction::*;
use neon::prelude::*;
use std::sync::Arc;

use serde::{ser::SerializeStruct, Serialize, Serializer};
use tk::pre_tokenizers::PreTokenizerWrapper;
use tk::PreTokenizedString;

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum JsPreTokenizerWrapper {
    Sequence(Vec<Arc<PreTokenizerWrapper>>),
    Wrapped(Arc<PreTokenizerWrapper>),
}

impl Serialize for JsPreTokenizerWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        match self {
            JsPreTokenizerWrapper::Sequence(seq) => {
                let mut ser = serializer.serialize_struct("Sequence", 2)?;
                ser.serialize_field("type", "Sequence")?;
                ser.serialize_field("pretokenizers", seq)?;
                ser.end()
            }
            JsPreTokenizerWrapper::Wrapped(inner) => inner.serialize(serializer),
        }
    }
}

impl<I> From<I> for JsPreTokenizerWrapper
where
    I: Into<PreTokenizerWrapper>,
{
    fn from(norm: I) -> Self {
        JsPreTokenizerWrapper::Wrapped(Arc::new(norm.into()))
    }
}

/// PreTokenizers
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PreTokenizer {
    #[serde(flatten)]
    pub pretok: Option<JsPreTokenizerWrapper>,
}

impl tk::PreTokenizer for PreTokenizer {
    fn pre_tokenize(&self, pretokenized: &mut PreTokenizedString) -> tk::Result<()> {
        match self.pretok.as_ref().ok_or("Uninitialized PreTokenizer")? {
            JsPreTokenizerWrapper::Sequence(seq) => {
                for pretokenizer in seq {
                    pretokenizer.pre_tokenize(pretokenized)?;
                }
            }
            JsPreTokenizerWrapper::Wrapped(pretokenizer) => {
                pretokenizer.pre_tokenize(pretokenized)?
            }
        };

        Ok(())
    }
}

declare_types! {
    pub class JsPreTokenizer for PreTokenizer {
        init(_) {
            // This should not be called from JS
            Ok(PreTokenizer { pretok: None })
        }
    }
}

/// byte_level(addPrefixSpace: bool = true)
fn byte_level(mut cx: FunctionContext) -> JsResult<JsPreTokenizer> {
    let mut byte_level = tk::pre_tokenizers::byte_level::ByteLevel::default();
    if let Some(add_prefix_space) = cx.extract_opt::<bool>(0)? {
        byte_level = byte_level.add_prefix_space(add_prefix_space);
    }

    let mut pretok = JsPreTokenizer::new::<_, JsPreTokenizer, _>(&mut cx, vec![])?;
    let guard = cx.lock();
    pretok.borrow_mut(&guard).pretok = Some(byte_level.into());
    Ok(pretok)
}

/// byte_level_alphabet()
fn byte_level_alphabet(mut cx: FunctionContext) -> JsResult<JsValue> {
    let chars = tk::pre_tokenizers::byte_level::ByteLevel::alphabet()
        .into_iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>();

    Ok(neon_serde::to_value(&mut cx, &chars)?)
}

/// whitespace()
fn whitespace(mut cx: FunctionContext) -> JsResult<JsPreTokenizer> {
    let mut pretok = JsPreTokenizer::new::<_, JsPreTokenizer, _>(&mut cx, vec![])?;
    let guard = cx.lock();
    pretok.borrow_mut(&guard).pretok =
        Some(tk::pre_tokenizers::whitespace::Whitespace::default().into());
    Ok(pretok)
}

/// whitespace_split()
fn whitespace_split(mut cx: FunctionContext) -> JsResult<JsPreTokenizer> {
    let mut pretok = JsPreTokenizer::new::<_, JsPreTokenizer, _>(&mut cx, vec![])?;
    let guard = cx.lock();
    pretok.borrow_mut(&guard).pretok = Some(tk::pre_tokenizers::whitespace::WhitespaceSplit.into());
    Ok(pretok)
}

/// bert_pre_tokenizer()
fn bert_pre_tokenizer(mut cx: FunctionContext) -> JsResult<JsPreTokenizer> {
    let mut pretok = JsPreTokenizer::new::<_, JsPreTokenizer, _>(&mut cx, vec![])?;
    let guard = cx.lock();
    pretok.borrow_mut(&guard).pretok = Some(tk::pre_tokenizers::bert::BertPreTokenizer.into());
    Ok(pretok)
}

/// metaspace(replacement: string = '_', addPrefixSpace: bool = true)
fn metaspace(mut cx: FunctionContext) -> JsResult<JsPreTokenizer> {
    let replacement = cx.extract_opt::<char>(0)?.unwrap_or('▁');
    let add_prefix_space = cx.extract_opt::<bool>(1)?.unwrap_or(true);

    let mut pretok = JsPreTokenizer::new::<_, JsPreTokenizer, _>(&mut cx, vec![])?;
    let guard = cx.lock();
    pretok.borrow_mut(&guard).pretok =
        Some(tk::pre_tokenizers::metaspace::Metaspace::new(replacement, add_prefix_space).into());
    Ok(pretok)
}

/// punctuation()
fn punctuation(mut cx: FunctionContext) -> JsResult<JsPreTokenizer> {
    let mut pretok = JsPreTokenizer::new::<_, JsPreTokenizer, _>(&mut cx, vec![])?;
    let guard = cx.lock();
    pretok.borrow_mut(&guard).pretok = Some(tk::pre_tokenizers::punctuation::Punctuation.into());
    Ok(pretok)
}

/// sequence()
fn sequence(mut cx: FunctionContext) -> JsResult<JsPreTokenizer> {
    let pretokenizers = cx.argument::<JsArray>(0)?.to_vec(&mut cx)?;
    let mut sequence = Vec::with_capacity(pretokenizers.len());

    pretokenizers
        .into_iter()
        .map(
            |pretokenizer| match pretokenizer.downcast::<JsPreTokenizer>().or_throw(&mut cx) {
                Ok(pretokenizer) => {
                    let guard = cx.lock();
                    let pretok = (*pretokenizer.borrow(&guard)).pretok.clone();
                    if let Some(pretokenizer) = pretok {
                        match pretokenizer {
                            JsPreTokenizerWrapper::Sequence(seq) => sequence.extend(seq),
                            JsPreTokenizerWrapper::Wrapped(inner) => sequence.push(inner),
                        }
                        Ok(())
                    } else {
                        cx.throw_error("Uninitialized PreTokenizer")
                    }
                }
                Err(e) => Err(e),
            },
        )
        .collect::<NeonResult<_>>()?;
    let mut pretok = JsPreTokenizer::new::<_, JsPreTokenizer, _>(&mut cx, vec![])?;
    let guard = cx.lock();
    pretok.borrow_mut(&guard).pretok = Some(JsPreTokenizerWrapper::Sequence(sequence));
    Ok(pretok)
}

/// char_delimiter_split(delimiter: string)
fn char_delimiter_split(mut cx: FunctionContext) -> JsResult<JsPreTokenizer> {
    let delimiter = cx.extract::<char>(0)?;

    let mut pretok = JsPreTokenizer::new::<_, JsPreTokenizer, _>(&mut cx, vec![])?;
    let guard = cx.lock();
    pretok.borrow_mut(&guard).pretok =
        Some(tk::pre_tokenizers::delimiter::CharDelimiterSplit::new(delimiter).into());

    Ok(pretok)
}

/// Register everything here
pub fn register(m: &mut ModuleContext, prefix: &str) -> NeonResult<()> {
    m.export_function(&format!("{}_ByteLevel", prefix), byte_level)?;
    m.export_function(
        &format!("{}_ByteLevel_Alphabet", prefix),
        byte_level_alphabet,
    )?;
    m.export_function(&format!("{}_Whitespace", prefix), whitespace)?;
    m.export_function(&format!("{}_WhitespaceSplit", prefix), whitespace_split)?;
    m.export_function(&format!("{}_BertPreTokenizer", prefix), bert_pre_tokenizer)?;
    m.export_function(&format!("{}_Metaspace", prefix), metaspace)?;
    m.export_function(
        &format!("{}_CharDelimiterSplit", prefix),
        char_delimiter_split,
    )?;
    m.export_function(&format!("{}_Punctuation", prefix), punctuation)?;
    m.export_function(&format!("{}_Sequence", prefix), sequence)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use tk::pre_tokenizers::sequence::Sequence;
    use tk::pre_tokenizers::whitespace::{Whitespace, WhitespaceSplit};
    use tk::pre_tokenizers::PreTokenizerWrapper;

    #[test]
    fn serialize() {
        let js_wrapped: JsPreTokenizerWrapper = Whitespace::default().into();
        let js_ser = serde_json::to_string(&js_wrapped).unwrap();

        let rs_wrapped = PreTokenizerWrapper::Whitespace(Whitespace::default());
        let rs_ser = serde_json::to_string(&rs_wrapped).unwrap();
        assert_eq!(js_ser, rs_ser);

        let js_pretok: PreTokenizer = serde_json::from_str(&rs_ser).unwrap();
        match js_pretok.pretok.unwrap() {
            JsPreTokenizerWrapper::Wrapped(pretok) => match pretok.as_ref() {
                PreTokenizerWrapper::Whitespace(_) => {}
                _ => panic!("Expected Whitespace"),
            },
            _ => panic!("Expected wrapped, not sequence."),
        }

        let js_seq: JsPreTokenizerWrapper =
            Sequence::new(vec![WhitespaceSplit.into(), Whitespace::default().into()]).into();
        let js_wrapper_ser = serde_json::to_string(&js_seq).unwrap();
        let rs_wrapped = PreTokenizerWrapper::Sequence(Sequence::new(vec![
            WhitespaceSplit.into(),
            Whitespace::default().into(),
        ]));
        let rs_ser = serde_json::to_string(&rs_wrapped).unwrap();
        assert_eq!(js_wrapper_ser, rs_ser);

        let js_seq = PreTokenizer {
            pretok: Some(js_seq),
        };
        let js_ser = serde_json::to_string(&js_seq).unwrap();
        assert_eq!(js_wrapper_ser, js_ser);

        let rs_seq = Sequence::new(vec![WhitespaceSplit.into(), Whitespace::default().into()]);
        let rs_ser = serde_json::to_string(&rs_seq).unwrap();
        assert_eq!(js_wrapper_ser, rs_ser);
    }
}
