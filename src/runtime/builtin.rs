use super::Value;

/// Standard helpers with explicit runtime dispatch for resource effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Bytes,
    EncodeUtf8,
    DecodeUtf8,
    ByteLen,
    Close,
}

impl Builtin {
    pub const ALL: [Self; 5] = [
        Self::Bytes,
        Self::EncodeUtf8,
        Self::DecodeUtf8,
        Self::ByteLen,
        Self::Close,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Bytes => "bytes",
            Self::EncodeUtf8 => "encode_utf8",
            Self::DecodeUtf8 => "decode_utf8",
            Self::ByteLen => "byte_len",
            Self::Close => "close",
        }
    }

    pub fn call(
        self,
        arguments: Vec<Value>,
        runtime: &mut impl super::Runtime,
    ) -> Result<Value, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "function '{}' expects 1 argument(s), got {}",
                self.name(),
                arguments.len()
            ));
        }
        let argument = arguments.into_iter().next().unwrap();
        match (self, argument) {
            (Self::Close, connection) => runtime.close(&connection).map(|()| Value::Null),
            (Self::Bytes, Value::Array(values)) => values
                .into_iter()
                .map(|value| {
                    if let Value::Integer(value) = value {
                        u8::try_from(value).map_err(|_| "bytes requires integers in 0..255".into())
                    } else {
                        Err("bytes requires integers in 0..255".into())
                    }
                })
                .collect::<Result<Vec<_>, String>>()
                .map(Value::Bytes),
            (Self::EncodeUtf8, Value::String(text)) => Ok(Value::Bytes(text.into_bytes())),
            (Self::DecodeUtf8, Value::Bytes(bytes)) => String::from_utf8(bytes)
                .map(Value::String)
                .map_err(|error| {
                    format!(
                        "decode_utf8: invalid UTF-8 at byte {}",
                        error.utf8_error().valid_up_to()
                    )
                }),
            (Self::ByteLen, Value::Bytes(bytes)) => i64::try_from(bytes.len())
                .map(Value::Integer)
                .map_err(|_| "byte length exceeds integer range".into()),
            (function, value) => Err(format!(
                "{} received unsupported {} argument",
                function.name(),
                value.type_name()
            )),
        }
    }
}
