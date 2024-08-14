// Borrowed code from: https://github.com/hashintel/hash/blob/bm/es/serde-hooks/libs/error-stack/src/serde.rs
use error_stack::serde::{HookContext, Serde, SerdeHooks};
use error_stack::{Frame, Report};
use serde::ser::SerializeSeq;
use serde::Serialize;

enum SerializableAttachment<'a> {
    Erased(Box<dyn erased_serde::Serialize + 'a>),
    Message(String),
}

impl<'a> Serialize for SerializableAttachment<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Erased(erased) => erased.serialize(serializer),
            Self::Message(data) => data.serialize(serializer),
        }
    }
}

enum EitherIterator<T, U>
where
    T: Iterator<Item = U::Item>,
    U: Iterator,
{
    Left(T),
    Right(U),
}

impl<T, U> Iterator for EitherIterator<T, U>
where
    T: Iterator<Item = U::Item>,
    U: Iterator,
{
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Left(left) => left.next(),
            Self::Right(right) => right.next(),
        }
    }
}

fn serialize_attachments<'a>(
    hooks: &'a SerdeHooks,
    frame: &'a Frame,
    context: &'a mut HookContext<Frame>,
) -> impl Iterator<Item = SerializableAttachment<'a>> + 'a {
    let mut attachments = hooks
        .call(frame, context)
        .map(SerializableAttachment::Erased)
        .peekable();

    let has_attachments = attachments.peek().is_some();
    if has_attachments {
        EitherIterator::Left(attachments)
    } else {
        let mut dbg_context = error_stack::fmt::Config::load(true);
        let dbg_context = dbg_context.context();

        let attachments = Report::invoke_debug_format_hook(|hooks| hooks.call(frame, dbg_context))
            .then(|| dbg_context.take_body())
            .unwrap_or_default();

        EitherIterator::Right(attachments.into_iter().map(SerializableAttachment::Message))
    }
}

pub(crate) struct SerializeAttachmentList<'a, 'b> {
    frames: &'a [&'b Frame],
}

impl<'a, 'b> SerializeAttachmentList<'a, 'b> {
    pub fn new(frames: &'a [&'b Frame]) -> Self {
        Self { frames }
    }
}

impl<'a, 'b> Serialize for SerializeAttachmentList<'a, 'b> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Report::invoke_serde_hook(|hooks| {
            let mut seq = serializer.serialize_seq(None)?;
            let mut context = HookContext::new(Serde::new());

            for frame in self.frames {
                for attachment in serialize_attachments(hooks, &frame, context.cast()) {
                    seq.serialize_element(&attachment)?;
                }
            }

            seq.end()
        })
    }
}
