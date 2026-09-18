use super::Value;

/// Standard helpers with explicit runtime dispatch for resource effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Bytes,
    EncodeUtf8,
    DecodeUtf8,
    ByteLen,
    Close,
    SetTimeout,
    Accept,
    LocalAddress,
}

impl Builtin {
    pub const ALL: [Self; 8] = [
        Self::Bytes,
        Self::EncodeUtf8,
        Self::DecodeUtf8,
        Self::ByteLen,
        Self::Close,
        Self::SetTimeout,
        Self::Accept,
        Self::LocalAddress,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Bytes => "bytes",
            Self::EncodeUtf8 => "encode_utf8",
            Self::DecodeUtf8 => "decode_utf8",
            Self::ByteLen => "byte_len",
            Self::Close => "close",
            Self::SetTimeout => "set_timeout",
            Self::Accept => "accept",
            Self::LocalAddress => "local_address",
        }
    }

    pub fn arity(self) -> usize {
        if self == Self::SetTimeout { 2 } else { 1 }
    }

    pub fn call(
        self,
        arguments: Vec<Value>,
        runtime: &mut impl super::Runtime,
    ) -> Result<Value, String> {
        if arguments.len() != self.arity() {
            return Err(format!(
                "function '{}' expects {} argument(s), got {}",
                self.name(),
                self.arity(),
                arguments.len()
            ));
        }
        let mut arguments = arguments.into_iter();
        let argument = arguments.next().unwrap();
        if self == Self::SetTimeout {
            let Value::Duration(milliseconds @ 1..=86_400_000) = arguments.next().unwrap() else {
                return Err("set_timeout requires a duration from 1ms through 24h".into());
            };
            return runtime
                .set_timeout(&argument, milliseconds)
                .map(|()| Value::Null);
        }
        match (self, argument) {
            (Self::Accept, listener) => runtime.accept(&listener),
            (Self::LocalAddress, connection) => {
                runtime.local_address(&connection).map(Value::String)
            }
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
